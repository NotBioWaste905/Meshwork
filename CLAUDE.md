# Meshwork

Task tracker where tasks are graph nodes and dependencies are visible by default.
Product vision: [MISSION.md](MISSION.md); architecture (C4): [docs/architecture.md](docs/architecture.md);
MVP plan and backlog: [TODO.md](TODO.md). Before changing the data model (node/edge types,
statuses) check MISSION.md § «Идея графа»: it fixes `BLOCKS` / `PART_OF` / `AFFECTS`,
`CHORE` nodes and no OR-dependencies.

## Layout

```
compose.yaml                 # backend + frontend + falkordb
scripts/                     # backup-falkordb.sh, restore-falkordb.sh
meshwork/backend_rs/         # Backend: Rust, axum 0.6, falkordb crate
  src/main.rs                #   bootstrap: env, FalkorDB connection, server
  src/lib.rs                 #   router (app()), CORS, HTTP handlers
  src/db.rs                  #   all Cypher queries
  src/frontier.rs            #   ready_tasks: pure frontier logic + unit tests
  src/schemas.rs             #   API contract: Task, NewTask, TaskUpdate, Status
  src/error.rs               #   AppError -> HTTP response
  tests/api.rs               #   integration tests against a live FalkorDB
meshwork/frontend/webapp/    # SvelteKit 2 + Svelte 5 (runes), Tailwind 4, @xyflow/svelte + dagre
  src/lib/api.ts             #   the only HTTP client, mirrors schemas.rs
  src/lib/graphStore.svelte.ts #  tasks of the open graph, shared by all graph screens
  src/routes/graphs/         #   graph list (known to this browser, localStorage); new/: create
  src/routes/graph/          #   +page.svelte: redirect to last/new graph
                             #   [id]/: graph view, board/, table/, task/[taskId]/
```

## Commands

```bash
docker compose up -d --build        # frontend :3000, backend :8000, FalkorDB :6379, FalkorDB UI :3001

# backend (needs FalkorDB: docker compose up -d falkordb)
cd meshwork/backend_rs
FALKORDB_URL=falkor://localhost:6379 cargo run
cargo clippy --all-targets -- -D warnings && cargo fmt
cargo test -- --include-ignored     # integration tests need FalkorDB on localhost:6379

# frontend
cd meshwork/frontend/webapp
npm ci && npm run dev               # :5173; backend allows CORS from :3000 and :5173
npm run check                       # svelte-check / TS
npm run lint                        # prettier --check + eslint
npm run format
```

Backend env: `FALKORDB_URL` (default `falkor://falkordb:6379`), `FALKORDB_GRAPH` (default
`meshwork`), `CORS_ALLOW_ORIGINS` (comma-separated or `*`, default
`http://localhost:3000,http://localhost:5173`). All user graphs live in one FalkorDB graph,
told apart by a `(:Graph {id})` node; tests use a throwaway FalkorDB graph `test_<uuid>`.

Frontend: `VITE_API_URL` (build time, default `http://localhost:8000`), see `.env.example`.

CI ([.github/workflows/ci.yml](.github/workflows/ci.yml)), on push and PR to `main`/`dev`:
backend `cargo fmt --check`, `clippy -D warnings`, integration tests against a FalkorDB
service container; frontend `npm run check`, `lint`, `build`. The FalkorDB version is pinned
in both `compose.yaml` and `ci.yml` — keep them equal.

## Data model (as in code)

```
(:Task {id, name, description, status, tags: [string]})-[:PART_OF]->(:Graph {id})
(:Task)-[:DEPENDS_ON]->(:Task)      # task depends on dep
(:Task)-[:ASSIGNED_TO]->(:User {id})
```

`status` is stored as the API string: `TODO`, `IN_PROGRESS`, `DONE`, `REVIEW`, `BLOCKED`
(`Status::as_str`); legacy `InProgress` is read via a serde alias.

`DEPENDS_ON` is the blocking edge MISSION.md calls `BLOCKS`: `(a)-[:DEPENDS_ON]->(b)` means
"b blocks a". Keep the name `DEPENDS_ON`, do not rename it.

API (`/v0/...`):
- `POST create_graph`; `GET :graph/all_tasks`
- `GET :graph/ready_tasks[?unassigned_only=true]` — frontier: `TODO` with all deps `DONE`
- `POST :graph/task/add/` — body `NewTask`, only `name` required; returns the created `Task`, 201
- `GET|PUT|DELETE :graph/task/:id` — PUT is a partial update, lists are replaced, unknown fields ignored
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
- Query values only via `.with_param(s)`; never format values into a Cypher string.
- Handlers that run several queries validate first (graph/tasks exist), then write, so a
  rejected request leaves no partial state. Keep that order.
- `#![recursion_limit = "256"]` in `lib.rs` is required: on recent rustc the falkordb async
  futures exceed the default limit.
- `npm ci` 404s → the global npm config may point at a corporate registry; the lockfile uses
  `registry.npmjs.org`, so pass `--registry=https://registry.npmjs.org`.
- Frontend: Svelte 5 runes (`$state`, `$props`), not Svelte 4 syntax. Tabs (prettier).

## Conventions

- Branches: `main` is stable, `dev` is the integration branch. Each TODO.md task gets a short
  branch from `dev` (`feat/…`, `fix/…`, `ci/…`, `docs/…`), merged into `dev` by PR after green
  CI. `dev` → `main` when a milestone (M0–M4 in TODO.md) is complete. Tick TODO.md items in
  the same branch.
- Commit messages: short, English, imperative (`Add …`, `Fix …`).
- CLAUDE.md files are in English; product docs (MISSION, architecture, TODO, operations) are in Russian.
- Update docs/architecture.md on significant architecture changes.
