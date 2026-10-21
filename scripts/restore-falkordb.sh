#!/bin/sh
# Replace the FalkorDB data volume with a backup made by backup-falkordb.sh.
#
#   scripts/restore-falkordb.sh path/to/falkordb-YYYYmmdd-HHMMSS.rdb
#
# The FalkorDB container must be stopped. Current data is DESTROYED.
# FALKORDB_VOLUME overrides the volume (default: meshwork_falkordb_data).
set -eu

backup=${1:?usage: $0 path/to/backup.rdb}
volume=${FALKORDB_VOLUME:-meshwork_falkordb_data}
image=falkordb/falkordb:v4.22.2

[ -f "$backup" ] || { echo "No such file: $backup" >&2; exit 1; }
if [ -n "$(docker ps -q --filter "volume=$volume")" ]; then
	echo "A container is using $volume; stop it first (docker compose stop falkordb)" >&2
	exit 1
fi

printf 'This replaces all data in volume %s with %s. Type "restore" to go on: ' "$volume" "$backup"
read -r answer
[ "$answer" = restore ] || { echo "Aborted"; exit 1; }

dir=$(cd "$(dirname "$backup")" && pwd)
name=$(basename "$backup")

# 1. Put the snapshot in place and drop the append-only files: when they exist,
#    Redis loads them instead of dump.rdb.
docker run --rm -v "$volume:/data" -v "$dir:/backup:ro" --entrypoint sh "$image" -c \
	"rm -rf /data/appendonlydir /data/dump.rdb && cp /backup/$name /data/dump.rdb"

# 2. Load the snapshot without AOF, then let Redis write a fresh AOF from it.
#    Starting with --appendonly yes right away would load an empty dataset.
tmp=meshwork-falkordb-restore
docker run -d --name "$tmp" -e BROWSER=0 -v "$volume:/var/lib/falkordb/data" "$image" >/dev/null
trap 'docker rm -f "$tmp" >/dev/null 2>&1 || true' EXIT
sleep 3
docker exec "$tmp" redis-cli CONFIG SET appendonly yes >/dev/null
tries=0
until docker exec "$tmp" redis-cli INFO persistence | tr -d '\r' |
	grep -q '^aof_rewrite_in_progress:0' &&
	docker exec "$tmp" redis-cli INFO persistence | tr -d '\r' |
	grep -q '^aof_rewrite_scheduled:0' &&
	docker exec "$tmp" redis-cli INFO persistence | tr -d '\r' |
	grep -q '^aof_last_bgrewrite_status:ok'; do
	tries=$((tries + 1))
	[ "$tries" -ge 60 ] && { echo "AOF rewrite did not finish" >&2; exit 1; }
	sleep 1
done
echo "Restored. Start it again: docker compose up -d falkordb"
