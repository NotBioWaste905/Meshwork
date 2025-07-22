use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Status {
    TODO,
    InProgress,
    DONE,
    REVIEW,
    BLOCKED,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub depends_on: Vec<String>,
    pub users: Vec<String>,
    pub tags: Vec<String>,
    pub status: Status,
    pub graph_id: String,
}
