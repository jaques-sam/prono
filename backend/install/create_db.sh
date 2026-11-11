#!/bin/bash
#
# Create `db_prono`, its tables and the app user on the SERVER's MariaDB, using
# the `[db]` user/pass from the local `secure_config.toml`. The app user can
# only log in from the NAS itself (also through an `ssh -L` tunnel) and from
# the LAN: remote tools (the CLI) reach MariaDB through the router's port
# forward, which the NAS sees as a `192.168.80.x` address.
# MariaDB on Synology only listens on its socket, so this runs over `ssh nas`;
# you are asked for the MariaDB *root* password once.
set -euo pipefail

CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/prono/secure_config.toml"
HERE="$(cd "$(dirname "$0")" && pwd)"
MYSQL=/usr/local/mariadb10/bin/mysql
SOCK=/run/mysqld/mysqld10.sock
SERVER=nas
HOSTS=(localhost 127.0.0.1 '192.168.80.%')

value() { sed -nE "s/^$1 *= *\"(.*)\"/\1/p" "$CONFIG"; }
sql_quote() { sed -e "s/\\\\/\\\\\\\\/g" -e "s/'/\\\\'/g"; }

user=$(value user | sql_quote)
pass=$(value pass | sql_quote)
[ -n "$user" ] && [ -n "$pass" ] || { echo "no user/pass in $CONFIG" >&2; exit 1; }

umask 077
sql=$(mktemp)
trap 'rm -f "$sql"; ssh $SERVER "rm -f .prono_db.sql" 2>/dev/null || true' EXIT
{
    echo "CREATE DATABASE IF NOT EXISTS db_prono;"
    echo "USE db_prono;"
    cat "$HERE/schema.sql"
    # Never open to every host: drop the wildcard user an older run or hand-made setup may have left.
    echo "DROP USER IF EXISTS '$user'@'%';"
    for host in "${HOSTS[@]}"; do
        echo "CREATE USER IF NOT EXISTS '$user'@'$host' IDENTIFIED BY '$pass';"
        # `IF NOT EXISTS` keeps an old password (or auth plugin): always set the current one.
        echo "ALTER USER '$user'@'$host' IDENTIFIED BY '$pass';"
        echo "GRANT SELECT, INSERT, UPDATE, DELETE ON db_prono.* TO '$user'@'$host';"
    done
} > "$sql"

ssh $SERVER "umask 077; cat > .prono_db.sql" < "$sql"
ssh -t $SERVER "$MYSQL -u root -p -S $SOCK < .prono_db.sql && echo 'db_prono ready'"
