//! Pure graph queries over a fetched task list.

use std::collections::HashMap;

use crate::model::{Status, Task};

/// Tasks an agent can pick up right now: `TODO` with every blocker `DONE`.
pub fn ready_tasks(tasks: &[Task], unassigned_only: bool) -> Vec<&Task> {
    let by_id: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    tasks
        .iter()
        .filter(|t| t.status == Status::Todo)
        .filter(|t| !unassigned_only || t.users.is_empty())
        .filter(|t| {
            t.depends_on.iter().all(|d| {
                by_id
                    .get(d.as_str())
                    .is_some_and(|b| b.status == Status::Done)
            })
        })
        .collect()
}

/// Tasks that list `id` as a blocker.
pub fn dependents<'a>(tasks: &'a [Task], id: &str) -> Vec<&'a Task> {
    tasks
        .iter()
        .filter(|t| t.depends_on.iter().any(|d| d == id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, status: Status, deps: &[&str], users: &[&str]) -> Task {
        Task {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            users: users.iter().map(|s| s.to_string()).collect(),
            tags: vec![],
            status,
            graph_id: "g".into(),
        }
    }

    fn ids(v: Vec<&Task>) -> Vec<&str> {
        v.into_iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn ready_requires_todo_and_done_blockers() {
        let tasks = vec![
            task("a", Status::Done, &[], &[]),
            task("b", Status::Todo, &["a"], &[]),
            task("c", Status::Todo, &["b"], &[]),
            task("d", Status::InProgress, &[], &[]),
            task("e", Status::Todo, &[], &[]),
        ];
        assert_eq!(ids(ready_tasks(&tasks, false)), vec!["b", "e"]);
    }

    #[test]
    fn ready_can_skip_assigned() {
        let tasks = vec![
            task("a", Status::Todo, &[], &["alice"]),
            task("b", Status::Todo, &[], &[]),
        ];
        assert_eq!(ids(ready_tasks(&tasks, true)), vec!["b"]);
        assert_eq!(ids(ready_tasks(&tasks, false)), vec!["a", "b"]);
    }

    #[test]
    fn missing_blocker_is_not_ready() {
        let tasks = vec![task("a", Status::Todo, &["ghost"], &[])];
        assert!(ready_tasks(&tasks, false).is_empty());
    }

    #[test]
    fn finds_dependents() {
        let tasks = vec![
            task("a", Status::Todo, &[], &[]),
            task("b", Status::Todo, &["a"], &[]),
            task("c", Status::Todo, &["a", "b"], &[]),
        ];
        assert_eq!(ids(dependents(&tasks, "a")), vec!["b", "c"]);
        assert!(dependents(&tasks, "c").is_empty());
    }
}
