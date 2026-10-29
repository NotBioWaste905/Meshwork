//! MCP tools. Deliberately workflow-shaped rather than a CRUD mirror of the
//! REST API, and there is no delete: agents may not destroy tasks.

use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    auth::Caller,
    graph::{self, Assigned},
    model::{ActorKind, NewActor, NewTask, Priority, Status, Task, TaskUpdate},
    view,
};

/// The tools. Stateless: every call acts through its [`Caller`], the backend client of the
/// token that made the request.
#[derive(Clone, Default)]
pub struct Meshwork;

impl Meshwork {
    pub fn new() -> Self {
        Self
    }
}

/// Tool outcome: errors go back to the model as `isError` results so it can react.
fn respond<T: Serialize>(res: Result<T, String>) -> Result<CallToolResult, ErrorData> {
    Ok(match res {
        Ok(v) => CallToolResult::success(vec![ContentBlock::json(v)?]),
        Err(msg) => CallToolResult::error(vec![ContentBlock::text(msg)]),
    })
}

fn ok<T: Serialize>(v: T) -> Result<CallToolResult, ErrorData> {
    respond(Ok::<_, String>(v))
}

fn fail(msg: String) -> Result<CallToolResult, ErrorData> {
    respond(Err::<(), _>(msg))
}

/// A write's outcome as a compact task: the agent knows what it sent, it needs the id and the
/// resulting state, not the description back.
fn respond_brief(res: Result<Task, String>) -> Result<CallToolResult, ErrorData> {
    match res {
        Ok(t) => ok(view::Brief::from(&t)),
        Err(e) => fail(e),
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateGraphArgs {
    /// The graph's goal, shown as its name in the UI, e.g. "Ship the billing v2 release".
    pub name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateActorArgs {
    /// Display name, unique among actors, e.g. "Andrew" or "codex".
    pub name: String,
    /// human (default) or agent.
    pub kind: Option<ActorKind>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GraphArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListTasksArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Include descriptions and timestamps. Default: false (compact: id, name, status,
    /// priority, tags, users, depends_on, pr_url). Prefer get_task_context for one task.
    pub full: Option<bool>,
}

/// Whose ready tasks `list_ready_tasks` returns.
#[derive(Deserialize, JsonSchema, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReadyFor {
    /// Tasks nobody is assigned to, plus tasks already assigned to you.
    #[default]
    Me,
    /// Only tasks nobody is assigned to.
    Unassigned,
    /// Every ready task, whoever is assigned.
    Anyone,
}

#[derive(Deserialize, JsonSchema)]
pub struct ReadyArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// me (default): unassigned tasks and tasks assigned to you; unassigned: only tasks
    /// nobody has; anyone: all ready tasks.
    #[serde(rename = "for")]
    pub for_: Option<ReadyFor>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TaskArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Task ID (UUID).
    pub task_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateTaskArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Short task title. Must not be empty.
    pub name: String,
    /// What needs doing and how to tell it is done.
    pub description: Option<String>,
    /// IDs of tasks that must be DONE before this one can start.
    pub blocked_by: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    /// Assignees: actor IDs or names (see list_actors; your own ID comes from whoami).
    pub users: Option<Vec<String>>,
    /// LOW, NORMAL (default) or HIGH.
    pub priority: Option<Priority>,
    /// Actor ID or name of who checks the result. Default: you.
    pub reporter: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateTaskArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Task ID (UUID).
    pub task_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<Status>,
    /// Replaces the whole tag list.
    pub tags: Option<Vec<String>>,
    /// Replaces the whole assignee list: actor IDs or names (see list_actors; yours: whoami).
    pub users: Option<Vec<String>>,
    /// LOW, NORMAL or HIGH.
    pub priority: Option<Priority>,
    /// URL of the task's pull request (http or https); empty string removes it.
    /// Set it when you open the PR and move the task to REVIEW.
    pub pr_url: Option<String>,
    /// Actor ID or name of who checks the result; empty string removes it.
    pub reporter: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CommentArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Task ID (UUID).
    pub task_id: String,
    /// The comment, Markdown. Signed as you.
    pub body: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct LinkArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// The task that blocks: it must be DONE first.
    pub blocker_id: String,
    /// The task that is blocked by `blocker_id`.
    pub blocked_id: String,
}

#[tool_router]
impl Meshwork {
    #[tool(
        description = "Return the actor you act as (from your bearer token): its id and name. \
                       Put that id in `users` when you take a task.",
        annotations(read_only_hint = true)
    )]
    async fn whoami(&self, caller: Caller) -> Result<CallToolResult, ErrorData> {
        respond(Ok::<_, String>(caller.actor))
    }

    #[tool(
        description = "List everyone who can be assigned to a task or be its reporter, people and \
                       agents, with their ids, kinds and task counts. Use it to assign work to \
                       someone else or to name a reporter; for yourself use whoami.",
        annotations(read_only_hint = true)
    )]
    async fn list_actors(&self, caller: Caller) -> Result<CallToolResult, ErrorData> {
        respond(caller.backend.list_actors().await)
    }

    #[tool(
        description = "Add a person or agent who is not in list_actors yet, so tasks can be \
                       assigned to them. Names are unique; returns the new actor with its id.",
        annotations(read_only_hint = false)
    )]
    async fn create_actor(
        &self,
        caller: Caller,
        Parameters(a): Parameters<CreateActorArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let actor = NewActor {
            name: a.name,
            kind: a.kind.unwrap_or(ActorKind::Human),
        };
        respond(caller.backend.create_actor(&actor).await)
    }

    #[tool(
        description = "List all graphs (projects) with their names and task counts, newest first. \
                       Call this first to find the graph_id of the project you work on.",
        annotations(read_only_hint = true)
    )]
    async fn list_graphs(&self, caller: Caller) -> Result<CallToolResult, ErrorData> {
        respond(caller.backend.list_graphs().await)
    }

    #[tool(
        description = "Create a new empty task graph named after its goal and return its ID.",
        annotations(read_only_hint = false)
    )]
    async fn create_graph(
        &self,
        caller: Caller,
        Parameters(a): Parameters<CreateGraphArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        respond(
            caller
                .backend
                .create_graph(a.name.as_deref().unwrap_or_default())
                .await
                .map(|id| json!({ "graph_id": id })),
        )
    }

    #[tool(
        description = "List every task in a graph, compact: id, name, status, priority, tags, \
                       users, depends_on (blocker ids), pr_url; empty fields are omitted. \
                       Descriptions only with full: true; for one task use get_task_context.",
        annotations(read_only_hint = true)
    )]
    async fn list_tasks(
        &self,
        caller: Caller,
        Parameters(a): Parameters<ListTasksArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let tasks = match caller.backend.all_tasks(&a.graph_id).await {
            Ok(tasks) => tasks,
            Err(e) => return fail(e),
        };
        if a.full.unwrap_or(false) {
            ok(tasks)
        } else {
            ok(view::briefs(&tasks))
        }
    }

    #[tool(
        description = "List tasks that can be started right now: status TODO and every blocker DONE, \
                       highest priority first, compact (no descriptions; empty fields omitted). \
                       By default the unassigned ones plus those already assigned to you. \
                       Start here when looking for work.",
        annotations(read_only_hint = true)
    )]
    async fn list_ready_tasks(
        &self,
        caller: Caller,
        Parameters(a): Parameters<ReadyArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let assigned = match a.for_.unwrap_or_default() {
            ReadyFor::Me => Assigned::NobodyOr(&caller.actor.id),
            ReadyFor::Unassigned => Assigned::Nobody,
            ReadyFor::Anyone => Assigned::Any,
        };
        let tasks = match caller.backend.all_tasks(&a.graph_id).await {
            Ok(tasks) => tasks,
            Err(e) => return fail(e),
        };
        ok(view::briefs(graph::ready_tasks(&tasks, assigned)))
    }

    #[tool(
        description = "Get a task with its full description, plus the tasks blocking it and the \
                       tasks it blocks (id, name, status, users). Call before starting a task.",
        annotations(read_only_hint = true)
    )]
    async fn get_task_context(
        &self,
        caller: Caller,
        Parameters(a): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = async {
            let tasks = caller.backend.all_tasks(&a.graph_id).await?;
            let task = tasks.iter().find(|t| t.id == a.task_id).ok_or_else(|| {
                format!("404: task {} not found in graph {}", a.task_id, a.graph_id)
            })?;
            let blockers = tasks.iter().filter(|t| task.depends_on.contains(&t.id));
            Ok(json!({
                "task": task,
                "blocked_by": view::neighbors(blockers),
                "blocks": view::neighbors(graph::dependents(&tasks, &task.id)),
            }))
        }
        .await;
        respond(res)
    }

    #[tool(
        description = "Take a task to work on: assigns you and sets IN_PROGRESS in one step, \
                       only if it is TODO, every blocker is DONE and nobody else is assigned. \
                       If it fails, someone was faster or the task is not ready: pick another \
                       from list_ready_tasks. Prefer this over update_task for starting work. \
                       Returns the task without its description.",
        annotations(read_only_hint = false)
    )]
    async fn claim_task(
        &self,
        caller: Caller,
        Parameters(a): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        respond_brief(caller.backend.claim_task(&a.graph_id, &a.task_id).await)
    }

    #[tool(
        description = "Create a task. Use blocked_by to declare which tasks must be DONE first. \
                       Returns the new task (with its id) without its description.",
        annotations(read_only_hint = false)
    )]
    async fn create_task(
        &self,
        caller: Caller,
        Parameters(a): Parameters<CreateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let new = NewTask {
            name: a.name,
            description: a.description.unwrap_or_default(),
            depends_on: a.blocked_by.unwrap_or_default(),
            users: a.users.unwrap_or_default(),
            tags: a.tags.unwrap_or_default(),
            priority: a.priority.unwrap_or_default(),
            pr_url: String::new(),
            reporter: a.reporter,
        };
        respond_brief(caller.backend.add_task(&a.graph_id, &new).await)
    }

    #[tool(
        description = "Partially update a task; omitted fields stay unchanged. \
                       To start work use claim_task; use status REVIEW when work is ready, DONE only when verified. \
                       Returns the updated task without its description.",
        annotations(read_only_hint = false, idempotent_hint = true)
    )]
    async fn update_task(
        &self,
        caller: Caller,
        Parameters(a): Parameters<UpdateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let update = TaskUpdate {
            name: a.name,
            description: a.description,
            tags: a.tags,
            users: a.users,
            status: a.status,
            priority: a.priority,
            pr_url: a.pr_url,
            reporter: a.reporter,
        };
        respond_brief(
            caller
                .backend
                .update_task(&a.graph_id, &a.task_id, &update)
                .await,
        )
    }

    #[tool(
        description = "Declare that blocker_id must be DONE before blocked_id can start. \
                       Rejected if it would create a cycle.",
        annotations(read_only_hint = false, idempotent_hint = true)
    )]
    async fn link_tasks(
        &self,
        caller: Caller,
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = caller
            .backend
            .connect(&a.graph_id, &a.blocker_id, &a.blocked_id)
            .await
            .map(|()| json!({ "linked": { "blocker": a.blocker_id, "blocked": a.blocked_id } }));
        respond(res)
    }

    #[tool(
        description = "Remove the blocking relation between two tasks.",
        annotations(read_only_hint = false, idempotent_hint = true)
    )]
    async fn unlink_tasks(
        &self,
        caller: Caller,
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = caller
            .backend
            .disconnect(&a.graph_id, &a.blocker_id, &a.blocked_id)
            .await
            .map(|()| json!({ "unlinked": { "blocker": a.blocker_id, "blocked": a.blocked_id } }));
        respond(res)
    }

    #[tool(
        description = "Read a task's comments, oldest first: notes, questions and hand-offs left by \
                       people and agents. Check them before starting or continuing a task.",
        annotations(read_only_hint = true)
    )]
    async fn list_comments(
        &self,
        caller: Caller,
        Parameters(a): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        respond(caller.backend.list_comments(&a.graph_id, &a.task_id).await)
    }

    #[tool(
        description = "Leave a comment on a task, signed as you: progress notes, a question for \
                       the reporter, or what the next person needs when you hand work over. \
                       Keep the task's description for what to do; comments are the conversation.",
        annotations(read_only_hint = false)
    )]
    async fn add_comment(
        &self,
        caller: Caller,
        Parameters(a): Parameters<CommentArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        respond(
            caller
                .backend
                .add_comment(&a.graph_id, &a.task_id, &a.body)
                .await,
        )
    }
}

#[tool_handler(
    name = "meshwork",
    instructions = "Meshwork is a task tracker where tasks are graph nodes and dependencies are explicit. \
                    Call whoami to learn your actor id (list_actors for everyone else), list_graphs to find the project's graph_id, list_ready_tasks to find unblocked work, get_task_context before starting, claim_task to take one, \
                    update_task to move status, and link_tasks to record new dependencies."
)]
impl ServerHandler for Meshwork {}
