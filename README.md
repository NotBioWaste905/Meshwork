# Meshwork 🕸

Meshwork is a task tracker where every task is a node in a graph. Dependencies
are visible by default instead of being reconstructed on calls: you see what
blocks what, what is ready to start, and what the whole graph leads to.

The same graph is shown three ways — **Graph**, **Board** and **Table** — and
none of them is a second-class view. See [MISSION.md](MISSION.md) for the
product idea (in Russian).

> **Status: early development.** Tasks, dependencies and the three views work
> against the real backend. Users, auth, the activity log, reviewers and graph
> metadata do not exist in the API yet; the UI shows mock data for them,
> marked with a `mock` badge. The full list is in [TODO.md](TODO.md).

## Stack

| Part | Tech |
|---|---|
| Backend | Rust, axum, [FalkorDB](https://www.falkordb.com/) (graph DB over the Redis protocol) |
| Frontend | SvelteKit 2, Svelte 5, Tailwind 4, @xyflow/svelte + dagre |
| Infra | Docker Compose, GitHub Actions |

## Quick start

Requires Docker with Compose.

```bash
git clone https://github.com/NotBioWaste905/Meshwork.git
cd Meshwork
docker compose up -d --build
```

| Service | URL |
|---|---|
| Web app | http://localhost:3000 |
| Backend API | http://localhost:8000 |
| FalkorDB browser | http://localhost:3001 |

## Development

Run FalkorDB in Docker and the apps locally:

```bash
docker compose up -d falkordb

# backend, http://localhost:8000
cd meshwork/backend_rs
FALKORDB_URL=falkor://localhost:6379 cargo run

# frontend, http://localhost:5173
cd meshwork/frontend/webapp
npm ci
npm run dev
```

Configuration:

| Variable | Where | Default |
|---|---|---|
| `FALKORDB_URL` | backend | `falkor://falkordb:6379` |
| `FALKORDB_GRAPH` | backend | `meshwork` |
| `CORS_ALLOW_ORIGINS` | backend, comma-separated or `*` | `http://localhost:3000,http://localhost:5173` |
| `VITE_API_URL` | frontend, build time | `http://localhost:8000` |

### Checks

```bash
# backend; integration tests need FalkorDB on localhost:6379
cd meshwork/backend_rs
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --include-ignored

# frontend
cd meshwork/frontend/webapp
npm run check
npm run lint
```

CI runs the same on every push and pull request to `main` and `dev`.

## API

All routes are under `/v0`; errors come back as `{"message": "..."}`.

| Method | Route | |
|---|---|---|
| `POST` | `/create_graph` | New empty graph, returns `{"ID": "..."}` |
| `GET` | `/:graph_id/all_tasks` | All tasks of a graph |
| `GET` | `/:graph_id/ready_tasks?unassigned_only=true` | The frontier: `TODO` tasks whose dependencies are all `DONE` |
| `POST` | `/:graph_id/task/add/` | Create a task; only `name` is required |
| `GET` `PUT` `DELETE` | `/:graph_id/task/:task_id` | Read, partially update, delete a task |
| `POST` | `/:graph_id/connect_nodes/` | `{"node_dependency", "node_dependee"}`: the dependency blocks the dependee; `409` if it would create a cycle |
| `POST` | `/:graph_id/disconnect_nodes/` | Remove that dependency |

A task looks like this:

```json
{
  "id": "4a482cdb-…",
  "name": "Write changelog",
  "description": "",
  "status": "TODO",
  "depends_on": ["…ids of the tasks blocking this one"],
  "users": ["Andrei Chirkin"],
  "tags": ["docs"],
  "graph_id": "b773cd2b-…"
}
```

`status` is one of `TODO`, `IN_PROGRESS`, `REVIEW`, `BLOCKED`, `DONE`.

## Repository layout

```
meshwork/backend_rs/        Rust backend (the active one)
meshwork/frontend/webapp/   SvelteKit web app
meshwork/backend/           Legacy Python backend, kept as reference only
docs/architecture.md        C4 diagrams
docs/design/                Screen designs the UI follows
MISSION.md                  Product idea
TODO.md                     Phased backlog
CLAUDE.md                   Notes for AI coding agents
```

## Contributing

`main` is stable and `dev` is the integration branch. Each task from
[TODO.md](TODO.md) gets a short branch from `dev` (`feat/…`, `fix/…`,
`docs/…`) and goes back into `dev` through a pull request once CI is green.
`dev` is merged into `main` when a chunk of work is complete.
