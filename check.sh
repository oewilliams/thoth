#!/bin/sh
# The whole gate, one command: format, tests, clippy as errors, docs.
set -e
cargo fmt --check
cargo test --locked --quiet
cargo clippy --locked --all-targets --quiet -- -D warnings
cargo doc --locked --no-deps --quiet
echo "all gates green"
