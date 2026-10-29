#!/bin/sh
# Issue an actor token for the Meshwork MCP server, save it and check it.
#
#   scripts/mcp-token.sh [--launchctl] [actor] [file]
#
# actor defaults to claude; file to .mcp-token in the repo root (gitignored, mode 600).
# An actor that does not exist yet is created, as an agent (MESHWORK_ACTOR_KIND=human for a
# person): the backend requires a token for every request, so a fresh install starts here.
# The token is checked against the backend (MESHWORK_API_URL, default http://localhost:8000).
# --launchctl (macOS) also runs `launchctl setenv MESHWORK_MCP_TOKEN …`: apps started from the
# Dock, like the Claude desktop app, read their environment from launchd, not from the shell.
# Older tokens of the actor stay valid: list and revoke them with meshwork-token.
set -eu

launchctl=false
if [ "${1:-}" = "--launchctl" ]; then
	launchctl=true
	shift
fi
root=$(cd "$(dirname "$0")/.." && pwd)
actor=${1:-claude}
file=${2:-$root/.mcp-token}
api=${MESHWORK_API_URL:-http://localhost:8000}
kind=${MESHWORK_ACTOR_KIND:-agent}

token=$(cd "$root" && docker compose exec -T backend \
	meshwork-token create "$actor" --kind "$kind" --label "mcp-token.sh on $(hostname -s)")
case $token in
mw_*) ;;
*)
	echo "meshwork-token did not print a token" >&2
	exit 1
	;;
esac

status=$(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $token" "$api/v0/me")
if [ "$status" != 200 ]; then
	echo "the backend at $api rejected the new token (HTTP $status)" >&2
	exit 1
fi

(umask 077 && printf '%s\n' "$token" >"$file")
echo "Saved the token for $actor to $file and checked it against $api." >&2

if $launchctl; then
	launchctl setenv MESHWORK_MCP_TOKEN "$token"
	echo "Set MESHWORK_MCP_TOKEN in launchd. Quit the Claude app (Cmd+Q) and open it again." >&2
fi

cat >&2 <<HINT

Claude Code reads MESHWORK_MCP_TOKEN when it starts (see .mcp.json), so restart it afterwards.
  sh/zsh:   export MESHWORK_MCP_TOKEN="\$(cat $file)"
  fish:     set -gx MESHWORK_MCP_TOKEN (cat $file)
  desktop:  launchctl setenv MESHWORK_MCP_TOKEN "\$(cat $file)", then Cmd+Q and reopen the app
HINT
