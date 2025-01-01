#!/usr/bin/env bash
# This scripts runs various CI-like checks in a convenient way.
set -eux

typos
lychee .
cargo fmt --all -- --check
cargo machete
cargo check --quiet --workspace --all-targets --all-features
cargo check --quiet --all-features --package prono-app --target wasm32-unknown-unknown
cargo clippy --quiet --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --quiet --workspace --all-targets --all-features
cargo test --quiet --workspace --doc
trunk build --config ./app
