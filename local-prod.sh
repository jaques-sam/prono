#!/usr/bin/env bash
# Tries the working tree locally against the PRODUCTION database (the NAS).
#
#   ./local-prod.sh             release-builds backend, app and CLI, starts the backend on
#                               http://localhost:8081 and runs the desktop app against it;
#                               the backend stops when the app is closed
#   ./local-prod.sh cli [args]  release-builds, then runs `prono-cli [args]` (no args: the menu)
#
# The database settings come from `<config dir>/prono/secure_config.toml`; `PRONO_DB_HOST`,
# `PRONO_DB_PORT`, `PRONO_DB_USER` and `PRONO_DB_PASS` override them (see backend/README.md).
# WARNING: everything you submit or change here is written to that database.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BACKEND_PORT=8081 # fixed in backend/src/main.rs
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release"
BACKEND_LOG="$BIN/local-prod-backend.log"

cargo build --release --bin prono-backend --bin prono-app --bin prono-cli

if [[ "${1:-}" == cli ]]; then
    shift
    exec "$BIN/prono-cli" "$@"
fi

if (echo >/dev/tcp/127.0.0.1/$BACKEND_PORT) 2>/dev/null; then
    echo "port $BACKEND_PORT is already in use" >&2
    exit 2
fi

"$BIN/prono-backend" >"$BACKEND_LOG" 2>&1 &
backend_pid=$!
trap 'kill "$backend_pid" 2>/dev/null || true' EXIT

# the backend exits at once when it cannot reach the database
for _ in $(seq 100); do
    if ! kill -0 "$backend_pid" 2>/dev/null; then
        echo "the backend stopped, see $BACKEND_LOG:" >&2
        tail -n 5 "$BACKEND_LOG" >&2
        exit 1
    fi
    (echo >/dev/tcp/127.0.0.1/$BACKEND_PORT) 2>/dev/null && break
    sleep 0.2
done

echo "backend (log: $BACKEND_LOG) and app run against the PRODUCTION database" >&2
PRONO_BACKEND_URL="http://localhost:$BACKEND_PORT" "$BIN/prono-app"
