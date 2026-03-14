#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

cargo fmt --all -- --check
cargo machete
cargo audit
cargo check --quiet --workspace --all-targets --all-features
cargo check --release --quiet --workspace --all-targets --all-features
cargo check --release --quiet --all-features --package prono-app --target wasm32-unknown-unknown
cargo clippy --release --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all -W clippy::pedantic
cargo tarpaulin --fail-under 80 --workspace --all-targets --all-features --out html
cargo test --release --doc --quiet --workspace --all-features
env -u NO_COLOR trunk build --quiet --config ./app
typos
lychee --accept 100..=103,200..=299,429 .
xdg-open tarpaulin-report.html
