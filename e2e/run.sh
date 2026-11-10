#!/usr/bin/env bash
# Brings the e2e stack up, runs e2e/e2e.py and tears down what it started.
# See e2e/README.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# E2E_SOURCE=worktree (default): test the working tree.
# E2E_SOURCE=head: test a clean `git archive HEAD` built in target/e2e-src (isolated from uncommitted edits).
SRC="$ROOT"
if [[ "${E2E_SOURCE:-worktree}" == head ]]; then
    SRC="$ROOT/target/e2e-src"
    rm -rf "$SRC" && mkdir -p "$SRC"
    git archive HEAD | tar -x -C "$SRC"
fi
export E2E_SRC="$SRC" E2E_BACKEND_BIN="$SRC/target/release/prono-backend"

SELENIUM_IMAGE="${SELENIUM_IMAGE:-docker.io/selenium/standalone-chromium:148.0}"
SELENIUM_NAME="${SELENIUM_NAME:-prono-e2e-selenium}"
WEB_PORT="${E2E_WEB_PORT:-8080}" # must be an origin the backend's CORS list allows (see backend/src/main.rs)
DIST="$ROOT/target/e2e-dist"
export E2E_OUT="${E2E_OUT:-$ROOT/target/e2e-shots}"
export E2E_DB_CONTAINER="${E2E_DB_CONTAINER:-prono-dev-db}"
export PRONO_DB_HOST="${PRONO_DB_HOST:-127.0.0.1}" PRONO_DB_PORT="${PRONO_DB_PORT:-13306}"
export PRONO_DB_USER="${PRONO_DB_USER:-prono-app}" PRONO_DB_PASS="${PRONO_DB_PASS:-devpass}"
mkdir -p "$E2E_OUT"

started_selenium=0
static_pid=""
cleanup() {
    [[ -n "$static_pid" ]] && kill "$static_pid" 2>/dev/null || true
    if [[ $started_selenium == 1 ]]; then podman rm -f "$SELENIUM_NAME" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

# 1. the dev database must be up (this script never touches the real NAS database)
podman exec "$E2E_DB_CONTAINER" true 2>/dev/null || { echo "database container '$E2E_DB_CONTAINER' is not running" >&2; exit 2; }
if (echo >/dev/tcp/127.0.0.1/8081) 2>/dev/null; then echo "port 8081 (backend) is already in use" >&2; exit 2; fi
if (echo >/dev/tcp/127.0.0.1/"$WEB_PORT") 2>/dev/null; then echo "port $WEB_PORT (web) is already in use" >&2; exit 2; fi

# 2. builds
(cd "$SRC" && cargo build --release --bin prono-backend && trunk build --config ./app --dist "$DIST")

# 3. browser
if ! curl -fs http://127.0.0.1:4444/status >/dev/null 2>&1; then
    podman rm -f "$SELENIUM_NAME" >/dev/null 2>&1 || true
    podman run -d --name "$SELENIUM_NAME" --network host --shm-size 2g \
        -e SE_NODE_MAX_SESSIONS=6 -e SE_NODE_OVERRIDE_MAX_SESSIONS=true "$SELENIUM_IMAGE" >/dev/null
    started_selenium=1
    for _ in $(seq 60); do curl -fs http://127.0.0.1:4444/status | grep -q '"ready": true' && break; sleep 1; done
fi

# 4. static web server for the trunk build (http://localhost:$WEB_PORT/index.html#dev)
python3 -m http.server "$WEB_PORT" --bind 127.0.0.1 -d "$DIST" >"$E2E_OUT/static.log" 2>&1 &
static_pid=$!
sleep 1

# 5. scenario (starts/stops the backend itself, needs Pillow)
python3 -I "$ROOT/e2e/e2e.py"
