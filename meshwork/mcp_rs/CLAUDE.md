# mcp_rs — Meshwork MCP server

Streamable-HTTP MCP server (`rmcp` 3.x, axum 0.8) for AI agents. A **thin layer over the REST
backend**: it never talks to FalkorDB, everything goes through the backend's `/v0/...`
(`BACKEND_URL`). Separate crate because `backend_rs` is on axum 0.6 and rmcp needs 0.8.

## Layout

```
src/main.rs      # env, startup (MCP_TOKENS required, otherwise exit 1)
src/lib.rs       # app(): /health open, /mcp behind bearer auth
src/auth.rs      # static bearer tokens, constant-time comparison
src/tools.rs     # MCP tools (#[tool_router]) — add new ones here
src/backend.rs   # REST client; backend errors {message} -> Err(String)
src/graph.rs     # pure graph queries (ready_tasks, dependents) + unit tests
src/model.rs     # hand-written mirror of backend_rs/src/schemas.rs
```

## Commands

```bash
cd meshwork/mcp_rs
MCP_TOKENS=dev-token cargo run      # :8080, backend defaults to http://localhost:8000
cargo clippy --all-targets -- -D warnings && cargo fmt
cargo test
```

Env: `MCP_TOKENS` (comma-separated, required), `BACKEND_URL`, `MCP_BIND` (default
`0.0.0.0:8080`), `MCP_ALLOWED_HOSTS` (extra Host headers; rmcp allows only loopback by
default — add the host name when deployed behind a domain or between compose containers).

Connecting from Claude Code: root `.mcp.json`, token in `MESHWORK_MCP_TOKEN`.

## Rules

- **Tools fit the agent's workflow, not a mirror of REST.** The model reads names and
  descriptions: a description says *when* to call the tool.
- **No deletion.** Agents do not delete tasks; don't add `delete_*` without an explicit decision.
- Return backend errors as an `isError` result (`respond()`), not a protocol error, so the
  model sees them and can react.
- Tool parameters are structs with `JsonSchema` and a doc comment on every field (they go
  into the schema).
- Backend contract (`schemas.rs`) changed → update `model.rs` by hand.
- Logic testable without the network lives in `graph.rs` with unit tests; a new tool with
  non-trivial logic goes there too.
- Changing behaviour "in the DB" is done in `backend_rs`, not here.
