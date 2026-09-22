#!/usr/bin/env bash
# Fast checks that Cloud Agents can run on Linux. Windows-only desktop
# integration tests are not started here.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${PATH}"

npm run build
npm run test:ui
node --test scripts/ai/select-release-tests.test.mjs
cargo fmt --all -- --check
cargo nextest run -p serverbond-core --locked -j 8 --status-level fail --final-status-level fail
cargo clippy -p serverbond-core --all-targets --locked -- -D warnings
