//! The change log: what changed on a task, worked out by comparing the task before and after a
//! write, so every field an update can touch is covered in one place.

use serde_json::{json, Value};

use crate::schemas::Task;

/// One change, before it is stored as an `(:Event)`. `from`/`to` are JSON values; long texts
/// (the description) are recorded as changed without their contents.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// `created`, `field`, `blocker_added`, `blocker_removed`.
    pub kind: &'static str,
    /// For `field`: which one (`name`, `status`, `users`, ...).
    pub field: Option<&'static str>,
    pub from: Value,
    pub to: Value,
}

impl Change {
    pub fn created() -> Change {
        Change {
            kind: "created",
            field: None,
            from: Value::Null,
            to: Value::Null,
        }
    }

    /// `blocker` now blocks the task (added) or no longer does.
    pub fn blocker(blocker_id: &str, added: bool) -> Change {
        Change {
            kind: if added {
                "blocker_added"
            } else {
                "blocker_removed"
            },
            field: None,
            from: Value::Null,
            to: json!(blocker_id),
        }
    }

    fn field(field: &'static str, from: Value, to: Value) -> Change {
        Change {
            kind: "field",
            field: Some(field),
            from,
            to,
        }
    }
}

/// What a write changed on a task. Blockers are left out: edge writes record them directly.
/// Lists compare as sets, so a reordering is no change.
pub fn diff(before: &Task, after: &Task) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut field = |name: &'static str, from: Value, to: Value| {
        if from != to {
            changes.push(Change::field(name, from, to));
        }
    };
    field("name", json!(before.name), json!(after.name));
    if before.description != after.description {
        field("description", Value::Null, json!("changed"));
    }
    field(
        "status",
        json!(before.status.as_str()),
        json!(after.status.as_str()),
    );
    field(
        "priority",
        json!(before.priority.as_str()),
        json!(after.priority.as_str()),
    );
    field("pr_url", json!(before.pr_url), json!(after.pr_url));
    field("reporter", json!(before.reporter), json!(after.reporter));
    field("users", sorted(&before.users), sorted(&after.users));
    field("tags", sorted(&before.tags), sorted(&after.tags));
    changes
}

fn sorted(list: &[String]) -> Value {
    let mut list = list.to_vec();
    list.sort();
    json!(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{Priority, Status};

    fn task() -> Task {
        Task {
            id: "t".into(),
            name: "Name".into(),
            description: "Long text".into(),
            depends_on: vec!["d".into()],
            users: vec!["a".into(), "b".into()],
            reporter: None,
            tags: vec!["x".into()],
            status: Status::Todo,
            priority: Priority::Normal,
            pr_url: String::new(),
            created_at: Some(1),
            updated_at: Some(1),
            created_by: None,
            position: None,
            graph_id: "g".into(),
        }
    }

    #[test]
    fn nothing_changed() {
        let mut after = task();
        after.users.reverse();
        after.updated_at = Some(2);
        after.depends_on.clear(); // edges are logged by the edge writes
        assert!(diff(&task(), &after).is_empty());
    }

    #[test]
    fn records_each_changed_field() {
        let after = Task {
            name: "Renamed".into(),
            description: "Other long text".into(),
            status: Status::InProgress,
            priority: Priority::High,
            pr_url: "https://x/pull/1".into(),
            reporter: Some("r".into()),
            users: vec!["c".into()],
            tags: vec![],
            ..task()
        };
        let changes = diff(&task(), &after);
        let fields: Vec<_> = changes.iter().map(|c| c.field.unwrap()).collect();
        assert_eq!(
            fields,
            [
                "name",
                "description",
                "status",
                "priority",
                "pr_url",
                "reporter",
                "users",
                "tags"
            ]
        );
        let status = &changes[2];
        assert_eq!(
            (&status.from, &status.to),
            (&json!("TODO"), &json!("IN_PROGRESS"))
        );
        // the description's text is not copied into the log
        assert_eq!(
            (&changes[1].from, &changes[1].to),
            (&Value::Null, &json!("changed"))
        );
        assert_eq!(changes[5].from, Value::Null);
        assert_eq!(changes[6].to, json!(["c"]));
    }
}
