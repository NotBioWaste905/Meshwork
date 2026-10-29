//! What the tools send back about tasks. Agents pay for every byte in tokens, so lists and
//! write confirmations carry a compact task, and only `get_task_context` (or `list_tasks` with
//! `full`) returns the full one with its description.

use serde::Serialize;

use crate::model::{Priority, Status, Task};

/// A task without its description and bookkeeping. Empty lists and an empty `pr_url` are left
/// out.
#[derive(Serialize, Debug, PartialEq)]
pub struct Brief<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub status: Status,
    pub priority: Priority,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub tags: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub users: &'a [String],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub depends_on: &'a [String],
    #[serde(skip_serializing_if = "str::is_empty")]
    pub pr_url: &'a str,
}

impl<'a> From<&'a Task> for Brief<'a> {
    fn from(t: &'a Task) -> Self {
        Self {
            id: &t.id,
            name: &t.name,
            status: t.status,
            priority: t.priority,
            tags: &t.tags,
            users: &t.users,
            depends_on: &t.depends_on,
            pr_url: &t.pr_url,
        }
    }
}

/// A task next to the one in focus (its blockers and what it blocks): enough to see whether it
/// is done and who has it. A goal node's `depends_on` alone can be dozens of ids.
#[derive(Serialize, Debug, PartialEq)]
pub struct Neighbor<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub status: Status,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub users: &'a [String],
}

impl<'a> From<&'a Task> for Neighbor<'a> {
    fn from(t: &'a Task) -> Self {
        Self {
            id: &t.id,
            name: &t.name,
            status: t.status,
            users: &t.users,
        }
    }
}

pub fn briefs<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Vec<Brief<'a>> {
    tasks.into_iter().map(Brief::from).collect()
}

pub fn neighbors<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Vec<Neighbor<'a>> {
    tasks.into_iter().map(Neighbor::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn task() -> Task {
        Task {
            id: "t1".into(),
            name: "Do it".into(),
            description: "A long description".into(),
            depends_on: vec![],
            users: vec!["claude".into()],
            reporter: Some("ann".into()),
            tags: vec![],
            status: Status::Todo,
            priority: Priority::High,
            pr_url: String::new(),
            created_at: Some(1),
            updated_at: Some(2),
            created_by: Some("ann".into()),
            position: None,
            graph_id: "g".into(),
        }
    }

    #[test]
    fn brief_drops_description_and_empty_fields() {
        let t = task();
        assert_eq!(
            serde_json::to_value(Brief::from(&t)).unwrap(),
            json!({ "id": "t1", "name": "Do it", "status": "TODO", "priority": "HIGH",
                    "users": ["claude"] })
        );
        let t = Task {
            depends_on: vec!["t0".into()],
            tags: vec!["m2".into()],
            pr_url: "https://example.com/pr/1".into(),
            ..task()
        };
        let v = serde_json::to_value(Brief::from(&t)).unwrap();
        assert_eq!(v["depends_on"], json!(["t0"]));
        assert_eq!(v["tags"], json!(["m2"]));
        assert_eq!(v["pr_url"], "https://example.com/pr/1");
    }

    #[test]
    fn neighbor_is_id_name_status_users() {
        let t = Task {
            depends_on: vec!["a".into(), "b".into()],
            ..task()
        };
        assert_eq!(
            serde_json::to_value(Neighbor::from(&t)).unwrap(),
            json!({ "id": "t1", "name": "Do it", "status": "TODO", "users": ["claude"] })
        );
    }
}
