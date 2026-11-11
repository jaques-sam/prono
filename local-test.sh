#!/usr/bin/env bash
# Tries the working tree entirely locally, on a COPY of the production database.
#
#   ./local-test.sh             copies the production database into a local MariaDB (podman), applies
#                               backend/install/schema.sql to it, release-builds backend, app and CLI,
#                               starts the backend on http://localhost:8081 and runs the desktop app on
#                               it; the backend stops when the app is closed
#   ./local-test.sh --keep-db   the same, on the local database as it is (keeps what you did to it)
#   ./local-test.sh cli [args]  release-builds, then runs `prono-cli [args]` (no args: the menu) on the
#                               local database (copied first when there is none yet)
#
# Needs: podman, cargo, python3 >= 3.11. No MySQL/MariaDB on the host: the dump and the restore run in
# the container (`docker.io/library/mariadb:10.11`, container `prono-local-db`, 127.0.0.1:23306).
# Production is only read (dump), with the settings of `<config dir>/prono/secure_config.toml`. The
# apps always get the local database through `PRONO_DB_*`, whatever your environment says.
# Remove the local database with `podman rm -f prono-local-db`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BACKEND_PORT=8081 # fixed in backend/src/main.rs
BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release"
BACKEND_LOG="$BIN/local-test-backend.log"

DB_CONTAINER=prono-local-db
DB_IMAGE=docker.io/library/mariadb:10.11 # the NAS runs MariaDB 10
DB_PORT=23306
DB_NAME=db_prono
DB_USER=prono-app
DB_PASS=localpass
DB_ROOT_PASS=localroot

die() { echo "$*" >&2; exit 1; }
port_in_use() { (echo >/dev/tcp/127.0.0.1/"$1") 2>/dev/null; }
local_sql() { podman exec -i "$DB_CONTAINER" mariadb -uroot -p"$DB_ROOT_PASS" "$@"; }

start_db() {
    if podman container exists "$DB_CONTAINER"; then
        podman start "$DB_CONTAINER" >/dev/null
    else
        ! port_in_use "$DB_PORT" || die "port $DB_PORT is already in use"
        podman run -d --name "$DB_CONTAINER" -p 127.0.0.1:"$DB_PORT":3306 \
            -e MARIADB_ROOT_PASSWORD="$DB_ROOT_PASS" -e MARIADB_DATABASE="$DB_NAME" \
            -e MARIADB_USER="$DB_USER" -e MARIADB_PASSWORD="$DB_PASS" \
            "$DB_IMAGE" --character-set-server=utf8mb4 --collation-server=utf8mb4_general_ci >/dev/null
    fi
    # (the first start answers on its socket before it listens on the port: ask over TCP)
    for _ in $(seq 120); do
        podman exec "$DB_CONTAINER" mariadb -h127.0.0.1 -u"$DB_USER" -p"$DB_PASS" -e 'SELECT 1' "$DB_NAME" \
            >/dev/null 2>&1 && return
        sleep 0.5
    done
    die "the local database does not start, see: podman logs $DB_CONTAINER"
}

has_copy() {
    [[ -n "$(local_sql -N -e "SHOW TABLES LIKE 'Survey'" "$DB_NAME")" ]]
}

# Replaces the local database with the production one, then brings it to the schema of this working tree.
copy_production_db() {
    local config="${XDG_CONFIG_HOME:-$HOME/.config}/prono/secure_config.toml"
    [[ -f "$config" ]] || die "no $config: it holds the production database settings to copy from"
    prod() { python3 -I -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["db"][sys.argv[2]])' "$config" "$1"; }
    local host port user pass
    host="$(prod host)" port="$(prod port)" user="$(prod user)" pass="$(prod pass)"

    echo "copying the production database from $host:$port ..." >&2
    local_sql -e "DROP DATABASE IF EXISTS $DB_NAME; CREATE DATABASE $DB_NAME CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci"
    MYSQL_PWD="$pass" podman exec -e MYSQL_PWD "$DB_CONTAINER" \
        mariadb-dump -h"$host" -P"$port" -u"$user" --single-transaction --skip-lock-tables "$DB_NAME" \
        | local_sql "$DB_NAME"
    local_sql "$DB_NAME" <backend/install/schema.sql
}

mode=app keep_db=0
case "${1:-}" in
cli) mode=cli keep_db=1; shift ;;
--keep-db) keep_db=1; shift ;;
esac

start_db
if [[ $keep_db == 0 ]] || ! has_copy; then copy_production_db; fi

# whatever the environment or `secure_config.toml` say: never the production database from here on
export PRONO_DB_HOST=127.0.0.1 PRONO_DB_PORT="$DB_PORT" PRONO_DB_USER="$DB_USER" PRONO_DB_PASS="$DB_PASS"

cargo build --release --bin prono-backend --bin prono-app --bin prono-cli

if [[ $mode == cli ]]; then
    exec "$BIN/prono-cli" "$@"
fi

! port_in_use "$BACKEND_PORT" || die "port $BACKEND_PORT is already in use"

"$BIN/prono-backend" >"$BACKEND_LOG" 2>&1 &
backend_pid=$!
trap 'kill "$backend_pid" 2>/dev/null || true' EXIT

for _ in $(seq 100); do
    if ! kill -0 "$backend_pid" 2>/dev/null; then
        echo "the backend stopped, see $BACKEND_LOG:" >&2
        tail -n 5 "$BACKEND_LOG" >&2
        exit 1
    fi
    port_in_use "$BACKEND_PORT" && break
    sleep 0.2
done

echo "backend (log: $BACKEND_LOG) and app run on the local database copy" >&2
PRONO_BACKEND_URL="http://localhost:$BACKEND_PORT" "$BIN/prono-app"
