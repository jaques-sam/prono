# End-to-end tests (browser)

Drives the web app (`trunk` build) in a real Chromium (Selenium standalone in
podman) against a real `prono-backend` and a **dev** MariaDB. Never use the
production database: the backend gets its DB settings from `PRONO_DB_*` only
and `XDG_CONFIG_HOME` points to an empty dir, so `secure_config.toml` is never read.

```sh
e2e/run.sh                      # test the working tree
E2E_SOURCE=head e2e/run.sh      # test a clean `git archive HEAD` (built in target/e2e-src)
E2E_OUT=/some/dir e2e/run.sh    # screenshots + backend.log (default target/e2e-shots)
```

## Prerequisites

- `podman`, `cargo`, `trunk`, the `wasm32-unknown-unknown` target, `python3` with Pillow.
- A running dev DB container (default `prono-dev-db`, `127.0.0.1:13306`, db `db_prono`,
  user `prono-app`/`devpass`, root `devroot`, schema from `backend/install/schema.sql`).
  Override with `E2E_DB_CONTAINER`, `E2E_DB_ROOT_PASS`, `PRONO_DB_HOST/PORT/USER/PASS`.
  **The test runs `DELETE FROM Users` in that database.**
- Free ports 8080 (static web server, an origin allowed by the backend's CORS list), 8081 (backend), 4444 (WebDriver).

`run.sh` builds, starts Selenium (`selenium/standalone-chromium:148.0`, host network, 6 parallel sessions;
it reuses a WebDriver already listening on 4444 but that must allow several sessions), serves the trunk
build, runs `e2e.py` (which starts/stops the backend itself, also to test outages) and removes what it started.

## How it works

- eframe 0.36 does **not** implement AccessKit on wasm (`accesskit_update: not currently implemented`
  in `eframe/src/web/app_runner.rs`), so the canvas has no DOM nodes. `wd.py` (stdlib W3C WebDriver
  client) therefore sends pointer/keyboard *actions* at fixed viewport coordinates (layout constants at
  the top of `e2e.py`; viewport is forced to 892x957). If the UI layout changes, adjust them.
- Software WebGL is needed in the container: `--use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader`
  (without it eframe panics "WebGL isn't supported").
- Assertions use the DB (`podman exec ... mariadb`), the REST calls the browser made (a `fetch` recorder
  injected with `Page.addScriptToEvaluateOnNewDocument`) and screenshot pixels (error banner is pink/red,
  timeline points are green discs). Each participant is a fresh browser session, hence a new device id.
- Do not `DELETE /actions` between pointer moves: the pointer would reset to (0, 0).

Not covered: text answers (the catalog has only date questions); the native desktop app.
