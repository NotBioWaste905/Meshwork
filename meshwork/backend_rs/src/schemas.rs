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

/// A task as returned by the API.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    /// IDs of the tasks this one depends on, i.e. the tasks blocking it.
    pub depends_on: Vec<String>,
    pub users: Vec<String>,
    pub tags: Vec<String>,
    pub status: Status,
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
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Status,
}

/// Request body for editing a task. Omitted fields are left unchanged; list
/// fields replace the stored list entirely. Unknown fields (`id`, `graph_id`,
/// ...) are ignored so a client can send back a whole task.
#[derive(Deserialize, Debug, Default)]
pub struct TaskUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub users: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub status: Option<Status>,
}

#[derive(Deserialize, Debug)]
pub struct ConnectNodesRequest {
    /// The blocking task.
    pub node_dependency: String,
    /// The task that depends on `node_dependency`.
    pub node_dependee: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGraphResponse {
    #[serde(rename = "ID")]
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageResponse {
    pub message: String,
}
