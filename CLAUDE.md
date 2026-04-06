# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Prono is a multi-platform survey application (desktop and web) written in Rust using the egui framework. It conducts surveys and stores answers in a MySQL database. The web app is deployed at https://jaques-sam.github.io/prono/.

## Build Commands
**Desktop app:**

```sh
cargo run --bin prono-app
# With logging:
RUST_LOG=debug cargo run --bin prono-app
```

**Web (local development):**
```sh
trunk serve --config ./app # Serves at http://127.0.0.1:8080
# Access at http://127.0.0.1:8080/index.html#dev (the #dev skips PWA caching)
```

**Web (production build):**
```sh
trunk build --release --config ./app
```

## Testing

**Run all tests:**
```sh
cargo test
# With coverage (requires 70% minimum):
cargo tarpaulin --fail-under 70 --workspace --all-targets --all-features
```

**Run tests for specific crate:**
```sh
cargo test -p <crate-name>
```

**Run single test:**
```sh
cargo test <test-name>
# With output:
cargo test <test-name> -- --nocapture
```

**Full CI validation (runs locally):**
```sh
./check.sh
```
This script runs: typos, lychee, fmt, machete, audit, check (debug + release + wasm), clippy, tarpaulin, doc tests, and trunk build.

## Code Quality

**Format:**
```sh
cargo fmt --all
```

**Lint:**
```sh
cargo clippy --release --workspace --all-targets --all-features -- -D warnings -W clippy::all -W clippy::pedantic
```

**Security audit:**
```sh
cargo audit
```

## Architecture

The codebase follows **Clean Architecture** (entities, ports, use cases, adapters) across all modules.

### Workspace Structure

- **app/**: egui-based GUI application
  - Compiles to both native (desktop) and wasm32 (web)
  - Uses clean architecture: adapters (GUI), entities (view models), main_native/main_wasm entry points

- **backend/**: New backend service under development
  - Being structured with clean architecture (entities, ports, use_cases, adapters)
  - Uses prono and prono_db crates
  - Currently minimal implementation in progress

- **prono/**: Core business logic library
  - Contains entities (Survey, Question, Answer, FileSurvey)
  - Ports define interfaces (repo traits, config readers, factories)
  - Use cases include fake_db for development/testing
  - **Key component**: `SyncPronoAdapter` bridges synchronous egui GUI with async database operations using mpsc channels and tokio
  - Implements prono_api::Surveys trait
  - Includes embedded survey JSON: `surveys/survey_spacex_starship.json`

- **prono_api/**: API trait definitions
  - Defines the `Surveys`, `Survey`, `Question`, `Answer` traits/types used by the GUI
  - Provides test utilities when `test-utils` feature is enabled

- **prono_db/**: Database adapter layer
  - Implements `repo::Db` trait using sqlx with MySQL backend
  - Requires manual database setup (see prono_db/README.md)
  - Tables: `Users`, `AnswerResponse`
  - Excluded from test coverage (.tarpaulin.toml)

- **generic/**: Shared utilities used across crates

### Key Architectural Patterns

**SyncPronoAdapter Pattern:**
The `prono` crate's `SyncPronoAdapter` solves the sync/async impedance mismatch:
- GUI (egui) is synchronous and must not block
- Database operations are async
- Solution: Background tokio task receives requests via `std::sync::mpsc`, performs async work, sends results back via per-request response channels
- GUI calls `request_*` methods, receives `Receiver<T>`, polls with `try_recv()` to stay non-blocking
- In debug builds, falls back to `FakeRepo` if DB initialization fails

**Clippy Configuration:**
Wildcard imports are explicitly allowed for clean architecture modules (`use entities::*;`, `use ports::*;`, etc.) per `.clippy.toml`.

## Configuration

**Database connection** via config file or environment variables:

Config file: `$HOME/.config/prono/config.toml`
```toml
[db]
host = "the_prono_db_host"
port = the_prono_db_port
user = "the_prono_db_user"
pass = "the_prono_db_password"
```

Or environment variables:
- `PRONO_DB_HOST`
- `PRONO_DB_PORT`
- `PRONO_DB_USER`
- `PRONO_DB_PASS`

**Secure files:**
This repo uses git-crypt. To work with encrypted files, obtain the GPG key from @jaques-sam and run:
```sh
gpg --import <KEY_FILENAME>
git-crypt unlock
```

## Development Setup

**Rust toolchain:**
```sh
rustup update  # Uses rust-toolchain.toml (channel 1.93)
rustup target add wasm32-unknown-unknown
```

**Tools:**
```sh
# Install binstall first:
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo tools:
cargo binstall trunk cargo-tarpaulin cargo-machete cargo-audit --secure
```

**Linux dependencies:**
```sh
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

**Fedora dependencies:**
```sh
dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel
```

## Working with Backend

The `backend/` crate is under active development with clean architecture structure:
- **entities/**: Domain entities
- **ports/**: Interfaces and error types (currently has `ports::Error` and `BackendResult<T>`)
- **use_cases/**: Business logic
- **adapters/**: External interface implementations

Backend reads config via `prono::factory::create_config_reader()` and initializes database using `SyncPronoAdapter::new_with_db_config::<prono_db::MysqlDb>()`.

## CI/CD

CI runs on push/PR via `.github/workflows/rust.yml`:
- Checks, tests (with 70% coverage requirement), fmt, clippy, audit
- Builds for multiple targets: Linux (x86_64, arm), macOS (x86_64, aarch64), Windows
- Wasm32 validation and trunk build
- Web deployment to GitHub Pages on `release` branch updates

## Notes

- Use edition 2024 for all crates
- Target wasm32-unknown-unknown for web builds
- The app includes PWA support via `assets/sw.js` (append `#dev` to URL to skip caching during development)
- Coverage excludes `app/src/adapters/*` and `prono_db/*` per `.tarpaulin.toml`
