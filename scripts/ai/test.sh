#!/usr/bin/env bash
# Fast checks that Cloud Agents can run on Linux. Windows-only desktop
# integration tests are not started here.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${PATH}"

npm run build
cargo fmt --all -- --check
cargo test -p serverbond-core --locked
cargo clippy -p serverbond-core --all-targets --locked -- -D warnings
