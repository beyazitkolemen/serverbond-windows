//! `ManagedChild`: a child process with redirected, size-capped output that
//! is killed reliably on drop (Windows job objects), plus `command()` which
//! hides background console windows.

use anyhow::{bail, Context, Result};
#[cfg(windows)]
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Child, Command, ExitStatus, Output, Stdio},
    time::{Duration, Instant},
};

pub fn command(executable: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(executable);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

pub struct ManagedChild {
    pub child: Child,
    pub spawned_at: Instant,
    #[cfg(windows)]
    _job: Job,
}

impl ManagedChild {
    pub fn spawn(mut cmd: Command, log_path: &Path) -> Result<Self> {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        cmd.stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log);
        Self::spawn_redirected(cmd)
    }

    pub fn spawn_with_stdin(
        mut cmd: Command,
        log_path: &Path,
        stdin: impl Into<Stdio>,
    ) -> Result<Self> {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        cmd.stdin(stdin).stdout(log.try_clone()?).stderr(log);
        Self::spawn_redirected(cmd)
    }

    fn spawn_redirected(mut cmd: Command) -> Result<Self> {
        #[allow(unused_mut)]
        let mut child = cmd
            .spawn()
            .context("Program başlatılamadı; Visual C++ x64 Runtime kurulumunu kontrol edin.")?;
        #[cfg(windows)]
        let job = match Job::attach(&child) {
            Ok(job) => job,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        Ok(Self {
            child,
            spawned_at: Instant::now(),
            #[cfg(windows)]
            _job: job,
        })
    }

    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<()> {
        let status = self.wait_status(timeout)?;
        if !status.success() {
            bail!("İşlem başarısız: {status}. Ayrıntılar Günlükler ekranında.");
        }
        Ok(())
    }

    fn terminate(&mut self) {
        #[cfg(windows)]
        self._job.terminate();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn wait_status(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }
            if started.elapsed() > timeout {
                self.terminate();
                bail!("İşlem zaman aşımına uğradı. Günlükleri kontrol edin.");
            }
            std::thread::sleep(Duration::from_millis(150));
        }
    }

    pub fn output(cmd: Command, timeout: Duration) -> Result<Output> {
        Self::output_with_stdin(cmd, timeout, Stdio::null())
    }

    pub fn output_with_stdin(
        mut cmd: Command,
        timeout: Duration,
        stdin: impl Into<Stdio>,
    ) -> Result<Output> {
        // Files avoid pipe deadlocks when a child writes more than the pipe buffer.
        let mut stdout = tempfile::tempfile()?;
        let mut stderr = tempfile::tempfile()?;
        cmd.stdin(stdin)
            .stdout(stdout.try_clone()?)
            .stderr(stderr.try_clone()?);
        let mut child = Self::spawn_redirected(cmd)?;
        let started = Instant::now();
        let status = loop {
            if stdout
                .metadata()?
                .len()
                .saturating_add(stderr.metadata()?.len())
                > 8 * 1024 * 1024
            {
                child.terminate();
                bail!("Komut çıktısı 8 MB sınırını aştı; işlem durduruldu.");
            }
            if let Some(status) = child.child.try_wait()? {
                break status;
            }
            if started.elapsed() > timeout {
                child.terminate();
                bail!("İşlem zaman aşımına uğradı; işlem durduruldu.");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        // A command may exit while a descendant still writes to these files.
        // Stop the entire tree before collecting a stable, combined-size result.
        child.terminate();
        let read = |file: &mut std::fs::File, limit: usize| -> Result<Vec<u8>> {
            if file.metadata()?.len() > limit as u64 {
                bail!("Komut çıktısı 8 MB sınırını aştı.");
            }
            file.seek(SeekFrom::Start(0))?;
            let mut bytes = Vec::new();
            file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > limit {
                bail!("Komut çıktısı 8 MB sınırını aştı.");
            }
            Ok(bytes)
        };
        let stdout = read(&mut stdout, 8 * 1024 * 1024)?;
        let stderr = read(&mut stderr, 8 * 1024 * 1024 - stdout.len())?;
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn reads_standard_input_without_putting_content_in_arguments() {
        use std::io::Write;
        let mut input = tempfile::tempfile().unwrap();
        input.write_all(b"stdin-only-marker\r\n").unwrap();
        input.rewind().unwrap();
        let mut cmd = command("cmd.exe");
        cmd.args(["/D", "/C", "findstr", "marker"]);
        assert!(!format!("{cmd:?}").contains("stdin-only-marker"));
        let output = ManagedChild::output_with_stdin(cmd, Duration::from_secs(5), input).unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("stdin-only-marker"));
    }

    #[test]
    fn captures_both_streams_and_nonzero_exit() {
        let mut cmd = command("cmd.exe");
        cmd.args(["/D", "/C", "echo output & echo error 1>&2 & exit /b 7"]);
        let output = ManagedChild::output(cmd, Duration::from_secs(5)).unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert!(String::from_utf8_lossy(&output.stdout).contains("output"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("error"));
    }

    #[test]
    fn timed_out_child_is_terminated_and_reaped() {
        let dir = tempfile::tempdir().unwrap();
        let mut cmd = command("ping.exe");
        cmd.args(["-n", "20", "127.0.0.1"]);
        let start = Instant::now();
        assert!(ManagedChild::output(cmd, Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn excessive_output_is_stopped_before_child_finishes() {
        let mut cmd = command(crate::terminal::powershell_path());
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", "$x = 'x' * 1048576; 1..10 | ForEach-Object { [Console]::Out.Write($x) }; Start-Sleep -Seconds 30"]);
        let error = ManagedChild::output(cmd, Duration::from_secs(20)).unwrap_err();
        // The size limit must stop the process before its 30-second tail runs.
        // A wall-clock assertion is unreliable on a shared CI runner.
        assert!(error.to_string().contains("8 MB"), "{error}");
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
// The owned kernel handle has no thread affinity and is closed exactly once.
unsafe impl Send for Job {}

#[cfg(windows)]
impl Job {
    fn terminate(&self) {
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            if TerminateJobObject(self.0, 1) == 0 {
                return; // Closing the owned job handle remains the fallback.
            }
            // Termination is asynchronous. Wait briefly for descendants to
            // release file handles before temporary files or install dirs move.
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
                if QueryInformationJobObject(
                    self.0,
                    JobObjectBasicAccountingInformation,
                    &mut info as *mut _ as *mut _,
                    std::mem::size_of_val(&info) as u32,
                    std::ptr::null_mut(),
                ) == 0
                    || info.ActiveProcesses == 0
                    || Instant::now() >= deadline
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    fn attach(child: &Child) -> Result<Self> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(handle);
                return Err(error.into());
            }
            Ok(Self(handle))
        }
    }
}

#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
