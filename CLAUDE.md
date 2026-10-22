# Meshwork

Task tracker where tasks are graph nodes and dependencies are visible by default.
Product vision: [MISSION.md](MISSION.md); architecture (C4): [docs/architecture.md](docs/architecture.md);
MVP plan: [TODO.md](TODO.md) (frozen; the live backlog is the Meshwork graph, see Conventions). Before changing the data model (node/edge types,
statuses) check MISSION.md § «Идея графа»: it fixes `BLOCKS` / `PART_OF` / `AFFECTS`,
`CHORE` nodes and no OR-dependencies.

## Layout

```
compose.yaml                 # backend + mcp + frontend + falkordb
scripts/                     # backup-falkordb.sh, restore-falkordb.sh
meshwork/backend_rs/         # Backend: Rust, axum 0.6, falkordb crate
  src/main.rs                #   bootstrap: env, FalkorDB connection, server
  src/lib.rs                 #   router (app()), CORS, HTTP handlers
  src/connection.rs          #   FalkorDB client, per-request reconnect ping, index setup
  src/db.rs                  #   all Cypher queries
  src/frontier.rs            #   ready_tasks: pure frontier logic + unit tests
  src/schemas.rs             #   API contract: Task, NewTask, TaskUpdate, Status
  src/error.rs               #   AppError -> HTTP response
  tests/api.rs               #   integration tests against a live FalkorDB
meshwork/frontend/webapp/    # SvelteKit 2 + Svelte 5 (runes), Tailwind 4, @xyflow/svelte + dagre
  src/lib/api.ts             #   the only HTTP client, mirrors schemas.rs
  src/lib/graphStore.svelte.ts #  tasks of the open graph, shared by all graph screens
  src/routes/graphs/         #   graph list (GET /v0/graphs); new/: create a named graph
  src/routes/graph/          #   +page.svelte: redirect to last/new graph
                             #   [id]/: graph view, board/, table/, task/[taskId]/
meshwork/mcp_rs/             # MCP server for AI agents (rmcp, axum 0.8), thin REST client of the backend
                             #   see meshwork/mcp_rs/CLAUDE.md; Claude Code connects via root .mcp.json
```

## Commands

```bash
docker compose up -d --build        # frontend :3000, backend :8000, MCP :8080, FalkorDB :6379, UI :3001

# backend (needs FalkorDB: docker compose up -d falkordb)
cd meshwork/backend_rs
FALKORDB_URL=falkor://localhost:6379 cargo run
cargo clippy --all-targets -- -D warnings && cargo fmt
cargo test -- --include-ignored     # integration tests need FalkorDB on localhost:6379

# MCP server (needs the backend on :8000)
cd meshwork/mcp_rs
MCP_TOKENS=dev-token cargo run      # :8080/mcp; for Claude Code: export MESHWORK_MCP_TOKEN=dev-token
cargo clippy --all-targets -- -D warnings && cargo fmt && cargo test

# frontend
cd meshwork/frontend/webapp
npm ci && npm run dev               # :5173; backend allows CORS from :3000 and :5173
npm run check                       # svelte-check / TS
npm run lint                        # prettier --check + eslint
npm run format
```

Backend env: `FALKORDB_URL` (default `falkor://falkordb:6379`), `FALKORDB_GRAPH` (default
`meshwork`), `CORS_ALLOW_ORIGINS` (comma-separated or `*`, default
`http://localhost:3000,http://localhost:5173`), `PORT` (default `8000`). All user graphs live in one FalkorDB graph,
told apart by a `(:Graph {id})` node; tests use a throwaway FalkorDB graph `test_<uuid>`.

Frontend: `VITE_API_URL` (build time, default `http://localhost:8000`), see `.env.example`.

CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)), on push and PR to `main`/`dev`:
backend `cargo fmt --check`, `clippy -D warnings`, integration tests against a FalkorDB
service container; mcp fmt, clippy, unit tests; frontend `npm run check`, `lint`, `build`. The FalkorDB version is pinned
in both `compose.yaml` and `ci.yml` — keep them equal.

## Data model (as in code)

```
(:Graph {id, name, created_at})     # name is the goal, "" for unnamed/older graphs
(:Task {id, name, description, status, tags: [string]})-[:PART_OF]->(:Graph)
(:Task)-[:DEPENDS_ON]->(:Task)      # task depends on dep
(:Task)-[:ASSIGNED_TO]->(:Actor {id, name, kind})   # kind: human | agent
```

Actors are global (not per graph). `name` is unique and never equal to another actor's `id`, so
task `users` resolve by id first, then by name. API-created actors get UUID ids; actors migrated
from the old free-text `(:User {id: <text>})` keep the text as id and name (`kind: agent` for
`claude`). The migration (`connection::migrate`, run at startup after `ensure_indexes`) is
idempotent; put future data migrations there too.

`status` is stored as the API string: `TODO`, `IN_PROGRESS`, `DONE`, `REVIEW`, `BLOCKED`
(`Status::as_str`); legacy `InProgress` is read via a serde alias.

`DEPENDS_ON` is the blocking edge MISSION.md calls `BLOCKS`: `(a)-[:DEPENDS_ON]->(b)` means
"b blocks a". Keep the name `DEPENDS_ON`, do not rename it.

API (`/v0/...`):
- `POST create_graph` — optional body `{name}` (no body / empty body = unnamed graph);
  returns `{ID, name}`
- `GET graphs` — all graphs, newest first: `[{id, name, task_count, done_count, blocked_count}]`
- `GET :graph` — one graph in the same shape; `PUT :graph` with `{name}` renames (blank → 400)
- `GET :graph/all_tasks`
- `GET :graph/ready_tasks[?unassigned_only=true]` — frontier: `TODO` with all deps `DONE`
- `POST :graph/task/add/` — body `NewTask`, only `name` required; returns the created `Task`, 201
- `GET|PUT|DELETE :graph/task/:id` — PUT is a partial update, lists are replaced, unknown fields ignored
- Task `users` are actor ids in responses; in requests each entry is an actor id or name,
  unknown → 404 (validated before any write)
- `GET actors` (by name), `POST actors` with `{name, kind?}` → 201 `Actor` (409 if the name is
  taken, 400 if blank); `GET|PUT actors/:id` (partial `{name?, kind?}`). No delete yet.
- `POST :graph/connect_nodes/`, `disconnect_nodes/` with `{node_dependency, node_dependee}` —
  dependency blocks dependee; on the canvas the edge goes dependency → dependee
- Errors are `{message}` with 400/404/500; 409 when a new dependency would close a
  `DEPENDS_ON` cycle (the dependency graph is always a DAG).

## Gotchas

- **FalkorDB data** lives in the `meshwork_falkordb_data` volume with AOF on. Backup/restore:
  `scripts/backup-falkordb.sh`, `scripts/restore-falkordb.sh`, see
  [docs/operations.md](docs/operations.md). Never run `docker compose down -v`, and never turn
  AOF on for a volume that only has `dump.rdb`: Redis loads an empty dataset.
- **FalkorDB, not Neo4j.** Cypher is nearly the same, the driver is `falkordb` (Redis protocol).
  No multi-query transactions: each `query()` is atomic, a handler as a whole is not.
- **Reconnect.** The client holds one multiplexed socket (redis `ConnectionManager`), which
  reconnects only after a command fails on it. So every `/v0/` request first runs a read-only
  ping (`connection::ensure_connection_layer`) that is retried once on a connection error;
  the handler's own queries, writes included, are never retried — don't add retries there
  (a retried write can apply twice). While FalkorDB is down, requests answer 500 after ~10 s.
- Indexes on `Task.id` and `Graph.id` are created at startup (`connection::ensure_indexes`,
  idempotent). A new label looked up by `id` needs its index added there.
- Query values only via `.with_param(s)`; never format values into a Cypher string.
- Handlers that run several queries validate first (graph/tasks exist), then write, so a
  rejected request leaves no partial state. Keep that order.
- `#![recursion_limit = "256"]` in `lib.rs` is required: on recent rustc the falkordb async
  futures exceed the default limit.
- `npm ci` 404s → the global npm config may point at a corporate registry; the lockfile uses
  `registry.npmjs.org`, so pass `--registry=https://registry.npmjs.org`.
- Frontend: Svelte 5 runes (`$state`, `$props`), not Svelte 4 syntax. Tabs (prettier).

## Conventions

- **Work is tracked in Meshwork itself**, graph "Finish the Meshwork MVP": find, take and hand
  in tasks through the Meshwork MCP server following the
  [meshwork-tasks skill](.claude/skills/meshwork-tasks/SKILL.md). TODO.md is frozen.
- Branches: `main` is stable, `dev` is the integration branch. Each task gets a short branch
  from `dev` (`feat/…`, `fix/…`, `ci/…`, `docs/…`), merged into `dev` by PR after green CI.
  `dev` → `main` when a milestone (tags `m0`–`m4` in the graph) is complete.
- Commit messages: short, English, imperative (`Add …`, `Fix …`).
- CLAUDE.md files are in English; product docs (MISSION, architecture, TODO, operations) are in Russian.
- Update docs/architecture.md on significant architecture changes.
