use std::{fs, io, path::Path};

// A release job keeps verified package archives between disposable test homes.
// Production installation still verifies SHA-256 before using every copy.
pub fn restore_package_cache(home: &Path) -> io::Result<()> {
    let Some(shared) = std::env::var_os("SERVERBOND_TEST_CACHE") else {
        return Ok(());
    };
    fs::create_dir_all(home.join("cache"))?;
    for item in fs::read_dir(shared)? {
        let item = item?;
        if item.file_type()?.is_file() {
            fs::copy(item.path(), home.join("cache").join(item.file_name()))?;
        }
    }
    Ok(())
}

pub fn save_package_cache(home: &Path) -> io::Result<()> {
    let Some(shared) = std::env::var_os("SERVERBOND_TEST_CACHE") else {
        return Ok(());
    };
    fs::create_dir_all(&shared)?;
    for item in fs::read_dir(home.join("cache"))? {
        let item = item?;
        if item.file_type()?.is_file() {
            let destination = Path::new(&shared).join(item.file_name());
            fs::copy(item.path(), destination)?;
        }
    }
    Ok(())
}
