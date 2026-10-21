#!/bin/sh
# Snapshot the running FalkorDB into a timestamped .rdb file on the host.
#
#   scripts/backup-falkordb.sh [dest_dir]
#
# dest_dir defaults to $MESHWORK_BACKUP_DIR or ~/meshwork-backups.
# MESHWORK_BACKUP_KEEP (default 14) newest backups are kept, older ones removed.
# FALKORDB_CONTAINER overrides the container (default: the compose service).
set -eu

dest=${1:-${MESHWORK_BACKUP_DIR:-$HOME/meshwork-backups}}
keep=${MESHWORK_BACKUP_KEEP:-14}
container=${FALKORDB_CONTAINER:-$(docker ps -q \
	--filter label=com.docker.compose.project=meshwork \
	--filter label=com.docker.compose.service=falkordb)}

if [ -z "$container" ]; then
	echo "FalkorDB container is not running" >&2
	exit 1
fi

cli() { docker exec "$container" redis-cli "$@" | tr -d '\r'; }

before=$(cli LASTSAVE)
# BGSAVE is refused while an AOF rewrite runs; retry for a while.
tries=0
until cli BGSAVE | grep -q 'Background saving started'; do
	tries=$((tries + 1))
	[ "$tries" -ge 30 ] && { echo "BGSAVE did not start" >&2; exit 1; }
	sleep 1
done
tries=0
while [ "$(cli LASTSAVE)" = "$before" ]; do
	tries=$((tries + 1))
	[ "$tries" -ge 120 ] && { echo "BGSAVE did not finish in 2 minutes" >&2; exit 1; }
	sleep 1
done
if ! cli INFO persistence | grep -q 'rdb_last_bgsave_status:ok'; then
	echo "BGSAVE failed" >&2
	exit 1
fi

mkdir -p "$dest"
file="$dest/falkordb-$(date +%Y%m%d-%H%M%S).rdb"
docker cp "$container:/var/lib/falkordb/data/dump.rdb" "$file"
echo "Saved $file"

# Keep the newest $keep backups.
ls -1t "$dest"/falkordb-*.rdb | tail -n +$((keep + 1)) | while read -r old; do
	rm -f "$old"
	echo "Removed $old"
done
