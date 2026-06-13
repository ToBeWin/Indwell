#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo test
node --check crates/indwell-console-pwa/app.js
node --check crates/indwell-console-pwa/sw.js

echo "non-hardware verification passed"
