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

/// How urgent a task is; `NORMAL` unless set.
#[derive(
    Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
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
    /// ID of the actor who asked for the task and checks the result.
    #[serde(default)]
    pub reporter: Option<String>,
    pub tags: Vec<String>,
    pub status: Status,
    pub priority: Priority,
    /// Link to the task's pull request; empty when there is none.
    pub pr_url: String,
    pub graph_id: String,
}

/// A graph as `GET /v0/graphs` returns it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphInfo {
    pub id: String,
    /// The graph's goal; empty for graphs created before graphs had names.
    pub name: String,
    pub task_count: i64,
    pub done_count: i64,
    pub blocked_count: i64,
}

#[derive(Serialize, Debug, Default)]
pub struct NewTask {
    pub name: String,
    pub description: String,
    pub depends_on: Vec<String>,
    pub users: Vec<String>,
    pub tags: Vec<String>,
    pub priority: Priority,
    pub pr_url: String,
    /// `None`: the backend makes the caller the reporter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actors_match_the_backend_contract() {
        let body = serde_json::to_value(NewActor {
            name: "Ann".into(),
            kind: ActorKind::Agent,
        })
        .unwrap();
        assert_eq!(body, serde_json::json!({ "name": "Ann", "kind": "agent" }));
        let actors: Vec<Actor> =
            serde_json::from_str(r#"[{"id":"a1","name":"Ann","kind":"human","task_count":2}]"#)
                .unwrap();
        assert_eq!(actors[0].task_count, 2);
    }

    #[test]
    fn decodes_graph_list_from_backend() {
        let body =
            r#"[{"id":"g1","name":"Ship it","task_count":3,"done_count":1,"blocked_count":0}]"#;
        let graphs: Vec<GraphInfo> = serde_json::from_str(body).unwrap();
        assert_eq!(graphs[0].name, "Ship it");
        assert_eq!(graphs[0].task_count, 3);
    }
}

/// What an actor is: a person or an AI agent.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    Human,
    Agent,
}

/// Body of `POST /v0/actors`.
#[derive(Serialize, Debug)]
pub struct NewActor {
    pub name: String,
    pub kind: ActorKind,
}

/// An actor as the backend returns it (`GET /v0/me`, `/v0/actors`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Actor {
    pub id: String,
    pub name: String,
    /// `human` or `agent`.
    pub kind: String,
    /// Tasks assigned to the actor, across all graphs.
    pub task_count: i64,
}
