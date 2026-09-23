import { spawnSync } from 'node:child_process';
import { availableParallelism } from 'node:os';

const mode = process.argv[2];
if (mode !== 'core' && mode !== 'workspace') {
  console.error('Usage: node scripts/ai/test-rust.mjs core|workspace [nextest options]');
  process.exit(2);
}

const configuredJobs = process.env.SERVERBOND_TEST_JOBS;
const jobs = configuredJobs === undefined ? Math.min(8, availableParallelism()) : Number(configuredJobs);
if (!Number.isInteger(jobs) || jobs < 1 || jobs > 32) {
  console.error('SERVERBOND_TEST_JOBS must be an integer between 1 and 32.');
  process.exit(2);
}

const scope = mode === 'core' ? ['-p', 'serverbond-core'] : ['--workspace'];
const command = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const result = spawnSync(command, [
  'nextest', 'run', ...scope, '--locked', '-j', String(jobs),
  '--status-level', 'fail', '--final-status-level', 'fail',
  ...process.argv.slice(3),
], { stdio: 'inherit' });

if (result.error) {
  console.error(result.error.message);
}
process.exit(result.status ?? 1);
