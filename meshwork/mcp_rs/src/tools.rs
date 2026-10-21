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
    backend::Backend,
    graph,
    model::{NewTask, Status, Task, TaskUpdate},
};

#[derive(Clone)]
pub struct Meshwork {
    backend: Backend,
}

impl Meshwork {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }
}

/// Tool outcome: errors go back to the model as `isError` results so it can react.
fn respond<T: Serialize>(res: Result<T, String>) -> Result<CallToolResult, ErrorData> {
    Ok(match res {
        Ok(v) => match ContentBlock::json(v) {
            Ok(c) => CallToolResult::success(vec![c]),
            Err(e) => return Err(e),
        },
        Err(msg) => CallToolResult::error(vec![ContentBlock::text(msg)]),
    })
}

#[derive(Deserialize, JsonSchema)]
pub struct GraphArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ReadyArgs {
    /// Graph ID (UUID).
    pub graph_id: String,
    /// Only return tasks nobody is assigned to. Default: true.
    pub unassigned_only: Option<bool>,
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
    /// Assignee IDs.
    pub users: Option<Vec<String>>,
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
    /// Replaces the whole assignee list.
    pub users: Option<Vec<String>>,
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
        description = "Create a new empty task graph and return its ID.",
        annotations(read_only_hint = false)
    )]
    async fn create_graph(&self) -> Result<CallToolResult, ErrorData> {
        respond(
            self.backend
                .create_graph()
                .await
                .map(|id| json!({ "graph_id": id })),
        )
    }

    #[tool(
        description = "List every task in a graph with its status, assignees and blockers.",
        annotations(read_only_hint = true)
    )]
    async fn list_tasks(
        &self,
        Parameters(a): Parameters<GraphArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        respond(self.backend.all_tasks(&a.graph_id).await)
    }

    #[tool(
        description = "List tasks that can be started right now: status TODO and every blocker DONE. \
                       Start here when looking for work.",
        annotations(read_only_hint = true)
    )]
    async fn list_ready_tasks(
        &self,
        Parameters(a): Parameters<ReadyArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = self.backend.all_tasks(&a.graph_id).await.map(|tasks| {
            graph::ready_tasks(&tasks, a.unassigned_only.unwrap_or(true))
                .into_iter()
                .cloned()
                .collect::<Vec<Task>>()
        });
        respond(res)
    }

    #[tool(
        description = "Get a task together with the tasks blocking it and the tasks it blocks.",
        annotations(read_only_hint = true)
    )]
    async fn get_task_context(
        &self,
        Parameters(a): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = async {
            let tasks = self.backend.all_tasks(&a.graph_id).await?;
            let task = tasks.iter().find(|t| t.id == a.task_id).ok_or_else(|| {
                format!("404: task {} not found in graph {}", a.task_id, a.graph_id)
            })?;
            let blockers: Vec<&Task> = tasks
                .iter()
                .filter(|t| task.depends_on.contains(&t.id))
                .collect();
            Ok(json!({
                "task": task,
                "blocked_by": blockers,
                "blocks": graph::dependents(&tasks, &task.id),
            }))
        }
        .await;
        respond(res)
    }

    #[tool(
        description = "Create a task. Use blocked_by to declare which tasks must be DONE first.",
        annotations(read_only_hint = false)
    )]
    async fn create_task(
        &self,
        Parameters(a): Parameters<CreateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let new = NewTask {
            name: a.name,
            description: a.description.unwrap_or_default(),
            depends_on: a.blocked_by.unwrap_or_default(),
            users: a.users.unwrap_or_default(),
            tags: a.tags.unwrap_or_default(),
        };
        respond(self.backend.add_task(&a.graph_id, &new).await)
    }

    #[tool(
        description = "Partially update a task; omitted fields stay unchanged. \
                       Use status IN_PROGRESS when you start, REVIEW when work is ready, DONE only when verified.",
        annotations(read_only_hint = false, idempotent_hint = true)
    )]
    async fn update_task(
        &self,
        Parameters(a): Parameters<UpdateTaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let update = TaskUpdate {
            name: a.name,
            description: a.description,
            tags: a.tags,
            users: a.users,
            status: a.status,
        };
        respond(
            self.backend
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
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = self
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
        Parameters(a): Parameters<LinkArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let res = self
            .backend
            .disconnect(&a.graph_id, &a.blocker_id, &a.blocked_id)
            .await
            .map(|()| json!({ "unlinked": { "blocker": a.blocker_id, "blocked": a.blocked_id } }));
        respond(res)
    }
}

#[tool_handler(
    name = "meshwork",
    instructions = "Meshwork is a task tracker where tasks are graph nodes and dependencies are explicit. \
                    Call list_ready_tasks to find unblocked work, get_task_context before starting, \
                    update_task to move status, and link_tasks to record new dependencies."
)]
impl ServerHandler for Meshwork {}
