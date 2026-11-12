# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Prono is a multi-platform survey application (desktop and web) written in Rust using the egui framework. It conducts surveys; the backend stores the answers in a MySQL/MariaDB database. The web app is deployed at https://jaques-sam.github.io/prono/.

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

**Everything locally (backend, app, CLI and a local copy of the production database):**

```sh
./local-test.sh             # copies the production DB into a local MariaDB (podman), release-builds, runs backend + app
./local-test.sh --keep-db   # the same, on the local DB as it is
./local-test.sh cli [args]  # prono-cli on the local DB (no args: the interactive menu)
```
Needs `podman`, `cargo` and `python3` >= 3.11 (no MySQL/MariaDB on the host). Production is only read, using
`secure_config.toml`; the apps always use the local DB (`127.0.0.1:23306`). Reset it with `podman rm -f prono-local-db`.

**Web (production build):**
```sh
trunk build --release --config ./app
```

## Testing

**Run all tests:**
```sh
cargo test
# With coverage (requires 80% minimum):
cargo tarpaulin --fail-under 80 --workspace --all-targets --all-features
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
./checks.sh
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

See the Architecture section of `README.md`. In short: every survey type is defined once, in `prono_api`, and both apps reach the database only through the backend's REST API.

### Workspace Structure

- **api/** (`prono_api`): the model shared by every binary, wasm-safe
  - `Survey`, `Question`, `AnswerKind`, `Answer`, `Submission`, `Results`
  - The survey definition: `Survey::from_json` (`api/src/definition.rs`). Surveys are uploaded to the backend's database (`prono-cli surveys upload`), never embedded; `api/surveys/*.json` are examples, also the test fixtures of the `samples` feature
  - There is no `access` key in a definition: every survey is opened by its pass phrase (`prono-cli surveys pass-phrase`, stored in the backend's database only), and the apps get surveys solely through `POST /api/surveys` with a pass phrase; `correct_answer`s (all or none) make a survey scored, and they never serialize, so they stay on the backend
  - The rules (`Submission::validate`, `check_user_name`, `Answer::is_valid_for`) and the REST contract (`surveys_path`, `answers_path`, `API_KEY`, `DEVICE_ID_HEADER`)
  - Question ids hash the question text and answer kind; stored answers reference them, so `existing_question_ids_are_stable` must keep passing

- **prono/**: server-side core
  - Use cases `upload_survey`, `submit`, `results` and `scores` (`service.rs`); the `Repo` also stores the survey definitions
  - `Repo` trait with `MysqlRepo` (runtime `sqlx` queries, one transaction per submission) and `MemoryRepo` (tests)
  - `DbConfig::load()`: `secure_config.toml` plus `PRONO_DB_*` env overrides

- **backend/**: `actix-web` server; `rest.rs` holds the handlers, the API key check and the error to HTTP status mapping

- **app/**: egui app for native and wasm32
  - `state.rs`: the flow as a state machine (no egui); `ui.rs`/`timeline.rs` only draw it and return `Action`s
  - `client.rs`: non-blocking REST client (`ehttp`), identical on both targets
  - No business rules in the UI: put them in `prono_api` or `state.rs`

- **cli/**: admin tool on top of `prono` (surveys, users): commands for scripts, and an interactive menu (`menu.rs`) when run without one; both go through `admin::run`

## Configuration

**Database connection** (backend and CLI only) via `<config dir>/prono/secure_config.toml`:
```toml
[db]
host = "the_prono_db_host"
port = 3306
user = "the_prono_db_user"
pass = "the_prono_db_password"
```

Each setting can be overridden by `PRONO_DB_HOST`, `PRONO_DB_PORT`, `PRONO_DB_USER`, `PRONO_DB_PASS`.

**App backend URL:** native uses `PRONO_BACKEND_URL` (default: production); web uses `http://localhost:8081` when the URL contains `#dev`.

**Secure files:**
This repo uses git-crypt. To work with encrypted files, obtain the GPG key from @jaques-sam and run:
```sh
gpg --import <KEY_FILENAME>
git-crypt unlock
```

## Development Setup

**Rust toolchain:**
```sh
rustup update  # Uses rust-toolchain.toml (channel 1.99)
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

## CI/CD

CI runs on push/PR via `.github/workflows/rust.yml`:
- Checks, tests (with 80% coverage requirement), fmt, clippy, audit
- Builds for multiple targets: Linux (x86_64, arm), macOS (x86_64, aarch64), Windows
- Wasm32 validation and trunk build
- Web deployment to GitHub Pages (`pages.yml`) on `main` branch updates, and to kr8ium.com (`kr8ium.yml`, over SSH) on `release` branch updates

## Notes

- Use edition >=2024 for all crates
- Target wasm32-unknown-unknown for web builds
- The app includes PWA support via `assets/sw.js` (append `#dev` to URL to skip caching during development)
- Coverage excludes drawing, I/O and entry-point files per `.tarpaulin.toml`
- Always add unit tests for all changes and bdd tests (./app/tests) for new app features
- Bump the version in the backend/install/spk/INFO file if anything in backend is changed
