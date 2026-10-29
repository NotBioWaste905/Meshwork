use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    #[default]
    Todo,
    // "InProgress" is how statuses were stored before the contract was fixed.
    #[serde(alias = "InProgress")]
    InProgress,
    Done,
    Review,
    Blocked,
}

impl Status {
    /// Value stored in the `status` property of a `:Task` node.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Todo => "TODO",
            Status::InProgress => "IN_PROGRESS",
            Status::Done => "DONE",
            Status::Review => "REVIEW",
            Status::Blocked => "BLOCKED",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

/// How urgent a task is. Tasks stored before priorities existed read as `Normal`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

impl Priority {
    /// Value stored in the `priority` property of a `:Task` node.
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "LOW",
            Priority::Normal => "NORMAL",
            Priority::High => "HIGH",
        }
    }

    pub fn parse(s: &str) -> Option<Priority> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

/// Where a task's node sits on the graph canvas, in canvas units (its top-left corner).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Request body for saving node positions: task ID -> position. Tasks left out keep theirs.
#[derive(Deserialize, Debug)]
pub struct LayoutUpdate {
    pub positions: BTreeMap<String, Position>,
}

/// A task as returned by the API.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    /// IDs of the tasks this one depends on, i.e. the tasks blocking it.
    pub depends_on: Vec<String>,
    /// IDs of the assigned actors.
    pub users: Vec<String>,
    /// ID of the actor who asked for the task and checks the result; by default whoever created
    /// it. `None` for tasks created without a token and older tasks.
    #[serde(default)]
    pub reporter: Option<String>,
    pub tags: Vec<String>,
    pub status: Status,
    pub priority: Priority,
    /// Link to the pull request doing the task; empty when there is none.
    pub pr_url: String,
    /// Milliseconds since the epoch; `None` for tasks created before this was recorded.
    #[serde(default)]
    pub created_at: Option<i64>,
    /// Last change to the task's fields, status, edges or assignees; `None` like `created_at`.
    #[serde(default)]
    pub updated_at: Option<i64>,
    /// ID of the actor whose token created the task; `None` without a token, for older tasks
    /// and once that actor was deleted.
    #[serde(default)]
    pub created_by: Option<String>,
    /// Saved canvas position; `None` until the node was placed, then the canvas lays it out.
    #[serde(default)]
    pub position: Option<Position>,
    pub graph_id: String,
}

/// Request body for creating a task. The ID is assigned by the server and the
/// graph is taken from the path.
#[derive(Deserialize, Debug)]
pub struct NewTask {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Actors to assign, each given by ID or by name.
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub priority: Priority,
    /// An `http(s)` URL or empty.
    #[serde(default)]
    pub pr_url: String,
    /// Actor ID or name; omitted or empty means the actor of the request's token, if any.
    #[serde(default)]
    pub reporter: Option<String>,
}

/// Request body for editing a task. Omitted fields are left unchanged; list
/// fields replace the stored list entirely. Unknown fields (`id`, `graph_id`,
/// ...) are ignored so a client can send back a whole task.
#[derive(Deserialize, Debug, Default)]
pub struct TaskUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub depends_on: Option<Vec<String>>,
    /// Actors to assign, each given by ID or by name.
    pub users: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    /// An `http(s)` URL, or empty to remove the link.
    pub pr_url: Option<String>,
    /// Actor ID or name, or empty to remove the reporter.
    pub reporter: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ConnectNodesRequest {
    /// The blocking task.
    pub node_dependency: String,
    /// The task that depends on `node_dependency`.
    pub node_dependee: String,
}

/// Request body for creating a graph. The body itself is optional.
#[derive(Deserialize, Debug, Default)]
pub struct NewGraph {
    /// The goal the graph leads to; empty when not given.
    #[serde(default)]
    pub name: String,
    /// ID of the space to create the graph in; the default space when not given.
    #[serde(default)]
    pub space: Option<String>,
}

/// Request body for editing a graph. Omitted fields are left unchanged.
#[derive(Deserialize, Debug, Default)]
pub struct GraphUpdate {
    pub name: Option<String>,
    /// ID of the space to move the graph to.
    pub space: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGraphResponse {
    #[serde(rename = "ID")]
    pub id: String,
    pub name: String,
    /// ID of the space the graph was created in.
    pub space: String,
}

/// Graph metadata with task counters, as returned by `GET /v0/graphs` and `GET /v0/:graph`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphInfo {
    pub id: String,
    /// Empty for graphs created before graphs had names.
    pub name: String,
    pub task_count: i64,
    pub done_count: i64,
    pub blocked_count: i64,
    /// Milliseconds since the epoch; `None` for the oldest graphs.
    #[serde(default)]
    pub created_at: Option<i64>,
    /// Latest change to the graph or any of its tasks.
    #[serde(default)]
    pub updated_at: Option<i64>,
    /// ID of the actor whose token created the graph, as for tasks.
    #[serde(default)]
    pub created_by: Option<String>,
    /// ID of the space the graph is in; `None` only for data the startup migration has not seen.
    #[serde(default)]
    pub space: Option<String>,
}

/// ID of the space that graphs created without a `space` go to, and that the migration moves
/// older graphs into. It cannot be deleted.
pub const DEFAULT_SPACE_ID: &str = "default";
/// Name the default space gets when it is created.
pub const DEFAULT_SPACE_NAME: &str = "Default";

/// A space: the top-level container of graphs, as returned by `GET /v0/spaces`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Space {
    pub id: String,
    pub name: String,
    /// Number of graphs in the space.
    pub graph_count: i64,
    /// Milliseconds since the epoch.
    #[serde(default)]
    pub created_at: Option<i64>,
    /// Latest rename (or creation).
    #[serde(default)]
    pub updated_at: Option<i64>,
    /// ID of the actor whose token created the space.
    #[serde(default)]
    pub created_by: Option<String>,
}

/// Request body for `POST /v0/spaces`.
#[derive(Deserialize, Debug)]
pub struct NewSpace {
    pub name: String,
}

/// Request body for `PUT /v0/spaces/:id`. Omitted fields are left unchanged.
#[derive(Deserialize, Debug, Default)]
pub struct SpaceUpdate {
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub message: String,
}

/// What an actor is. Humans and agents are assigned the same way; the kind is
/// for display and, later, for access rules.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    #[default]
    Human,
    Agent,
}

impl ActorKind {
    /// Value stored in the `kind` property of an `:Actor` node.
    pub fn as_str(self) -> &'static str {
        match self {
            ActorKind::Human => "human",
            ActorKind::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<ActorKind> {
        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
    }
}

/// Someone who works on tasks: a person or an AI agent. Actors are global,
/// shared by all graphs; `Task.users` holds actor IDs.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Actor {
    /// A UUID for actors created through the API; actors migrated from the old
    /// free-text assignees keep that text as their ID.
    pub id: String,
    /// Unique, and never equal to another actor's ID, so a task's `users` may
    /// name an actor by either.
    pub name: String,
    pub kind: ActorKind,
    /// Number of tasks assigned to the actor, across all graphs.
    pub task_count: i64,
}

/// Request body for creating an actor.
#[derive(Deserialize, Debug)]
pub struct NewActor {
    pub name: String,
    #[serde(default)]
    pub kind: ActorKind,
}

/// Request body for editing an actor. Omitted fields are left unchanged.
#[derive(Deserialize, Debug, Default)]
pub struct ActorUpdate {
    pub name: Option<String>,
    pub kind: Option<ActorKind>,
}

/// Request body for merging an actor into another one.
#[derive(Deserialize, Debug)]
pub struct MergeActors {
    /// The actor that stays, by ID or by name.
    pub into: String,
}

/// A note an actor left on a task, as returned by the API.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Comment {
    pub id: String,
    /// Markdown.
    pub body: String,
    /// ID of the actor who wrote it; `None` once that actor was deleted.
    pub author: Option<String>,
    /// Milliseconds since the epoch.
    pub created_at: i64,
}

/// Request body for adding a comment; the author is the actor of the request's token.
#[derive(Deserialize, Debug)]
pub struct NewComment {
    pub body: String,
}

/// An API token as listed by the `meshwork-token` CLI. The token itself is never stored, only
/// its hash, and is never returned after it was issued.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TokenInfo {
    pub id: String,
    /// What the token is for, e.g. "claude on the laptop"; may be empty.
    pub label: String,
    /// Milliseconds since the epoch.
    pub created_at: i64,
    pub actor_id: String,
    pub actor_name: String,
}

/// One of the caller's own tokens, as `GET /v0/me/tokens` lists it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OwnToken {
    #[serde(flatten)]
    pub info: TokenInfo,
    /// The token this request was made with.
    pub current: bool,
}

/// Request body for issuing the caller another token.
#[derive(Deserialize, Debug, Default)]
pub struct NewToken {
    /// What the token is for, e.g. "laptop browser"; may be empty.
    #[serde(default)]
    pub label: String,
}

/// A newly issued token: the only time the token itself is shown.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IssuedToken {
    pub token: String,
    pub info: TokenInfo,
}

/// One entry of a task's change log, as `GET /v0/:graph/task/:id/events` returns it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Event {
    pub id: String,
    /// `created`, `field` (see `field`), `blocker_added` or `blocker_removed` (`to` is the
    /// blocking task's ID).
    pub kind: String,
    /// For `field` events: `name`, `description`, `status`, `priority`, `pr_url`, `reporter`,
    /// `users` or `tags`.
    pub field: Option<String>,
    /// Value before; `null` when there was none or it is not recorded (the description).
    pub from: serde_json::Value,
    /// Value after.
    pub to: serde_json::Value,
    /// ID of the actor who made the change; `None` without a token or once the actor is gone.
    pub actor: Option<String>,
    /// Milliseconds since the epoch.
    pub at: i64,
}
