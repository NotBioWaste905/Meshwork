---
name: meshwork-tasks
description: How to find, take, do and hand in work on the Meshwork project through the Meshwork MCP server (mcp__meshwork__* tools). Use whenever you are asked to work on Meshwork tasks, pick the next task, report progress, add a task to the backlog, or when work you do in this repo corresponds to a task in the graph. The backlog lives in the Meshwork graph, not in TODO.md.
---

# Working on Meshwork through Meshwork

The project's backlog is a graph in Meshwork itself. TODO.md is frozen: never add or tick items
there. Every piece of work — yours, a subagent's or a human's — is a task in the graph, and its
status in the graph is the truth.

- Graph: **"Finish the Meshwork MVP"**, id `3ea37679-5d8d-4cec-b282-07aff4a76628`
  (once `list_graphs` exists in MCP, find it by name instead).
- Goal node: "Finish the Meshwork MVP" (`a81b9a62-2787-411f-927c-da9952566910`). Every task
  ends up blocking it directly or through other tasks.
- UI: http://localhost:3000/graph/3ea37679-5d8d-4cec-b282-07aff4a76628
- Tools: `mcp__meshwork__*`. If they are missing, the server is not connected (needs
  `MESHWORK_MCP_TOKEN` in the app's environment and the compose stack up). A 401 in the
  connection error names the cause; point the user to `scripts/mcp-token.sh` and the MCP section
  of docs/operations.md (restart needed; the desktop app reads launchd's env, not the shell's).
  Don't fall back to editing TODO.md. The REST API on :8000 is a last resort for bulk edits.

## Conventions in the graph

- **Name**: short English title. **Description**: Russian, what to do and how to tell it is done.
- **Tags**: milestone (`m0`…`m4`, `post-mvp`) + area (`backend`, `frontend`, `mcp`, `ops`,
  `docs`, `process`, `security`).
- **Priority**: `NORMAL` by default; `HIGH` for what should go first (`list_ready_tasks` puts
  it first), `LOW` for nice-to-haves.
- **Assignee** (`users`): `claude` for any Claude agent, the person's name for a human.
- **Edges**: `link_tasks(blocker, blocked)` — "blocker must be DONE before blocked starts".
  No cycles (409). Only real blocking relations; "would be nice first" is not an edge.
- **Statuses**: `TODO` → `IN_PROGRESS` (taken) → `REVIEW` (PR open) → `DONE` (PR merged into
  `dev`). `BLOCKED` = started but stuck on something outside the graph (say what in the
  description).

## The loop

1. **Find work.** `list_ready_tasks(graph_id)` — `TODO`, unassigned, all blockers `DONE`.
   Prefer the lowest milestone tag, then `HIGH` priority, then the task that unblocks the most (`get_task_context`
   → `blocks`). If the user named a task, use that one. Skip tasks that need the user's machine
   or accounts (`ops` like cron/launchd, secrets) unless the user asked — propose them instead.
2. **Read context.** `get_task_context(graph_id, task_id)`: description, blockers' results,
   what it unblocks. Read the relevant code and CLAUDE.md before deciding the approach.
3. **Take it.** There is no atomic claim yet, so: re-read the task right before taking it; if
   `users` is non-empty or status is not `TODO`, pick another. Then
   `update_task(status: IN_PROGRESS, users: ["claude"])`. When dispatching subagents, each takes
   its own task this way; never give two agents the same task.
4. **Work.** Branch from `origin/dev` (`feat/…`, `fix/…`, `docs/…`, `ci/…`, `chore/…`), in a
   worktree if other work is going on in the main checkout. Follow CLAUDE.md: tests, fmt,
   clippy `-D warnings`, `npm run check` / `lint` / `build` for the parts you touched.
5. **Discovered work goes into the graph, not into the PR.** Out-of-scope bug or follow-up →
   `create_task` (tags, Russian description, `blocked_by` if any) and, if it blocks something,
   `link_tasks`. If it blocks *your* task, link it, put your task back to `TODO` with
   `users: []`, and say so in your report.
6. **Hand in.** Commit (short imperative English message), push, open a PR into `dev`. Then
   `update_task(status: REVIEW, pr_url: <PR URL>, description: <old description + result block>)`. Until task
   artifacts exist, the result goes at the end of the description:

   ```
   ---
   Итог (claude, YYYY-MM-DD): что сделано, как проверено, что осталось/оговорки.
   PR: https://github.com/NotBioWaste905/Meshwork/pull/N
   Ветка: feat/…
   ```

   Keep the original description above the block intact; `update_task` replaces the whole text.
7. **Done.** `DONE` only after the PR is merged into `dev` (check `gh pr view N --json state`).
   Agents merge only when the user asked for it. After merging, set `DONE` and check
   `list_ready_tasks` — newly unblocked tasks are the next candidates.

## Rules

- Never delete tasks (there is no tool for it on purpose). Obsolete task → `DONE` with a
  result block saying why it is not needed, after the user agrees.
- Changing scope or edges of tasks you don't own: fine for obvious fixes (typo, missing edge);
  for re-planning (dropping a dependency, merging tasks) ask the user first and note the
  decision in the description.
- Don't leave tasks `IN_PROGRESS` when you stop: either `REVIEW` with a PR, or back to `TODO`
  with `users: []` and a note on what was done so far.
- Report to the user in terms of the graph: which tasks you took, which moved to `REVIEW` /
  `DONE`, which you created, and what is ready next.
