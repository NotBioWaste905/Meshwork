//! Mirror of the backend API contract (`backend_rs/src/schemas.rs`).
//! Keep in sync by hand: the MCP server talks to the backend over REST only.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Todo,
    InProgress,
    Done,
    Review,
    Blocked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    /// IDs of the tasks blocking this one.
    pub depends_on: Vec<String>,
    /// IDs of the assigned actors (`/v0/actors`).
    pub users: Vec<String>,
    pub tags: Vec<String>,
    pub status: Status,
    pub graph_id: String,
}

#[derive(Serialize, Debug, Default)]
pub struct NewTask {
    pub name: String,
    pub description: String,
    pub depends_on: Vec<String>,
    pub users: Vec<String>,
    pub tags: Vec<String>,
}

/// Fields left as `None` are omitted from the body and so stay unchanged.
#[derive(Serialize, Debug, Default)]
pub struct TaskUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}
