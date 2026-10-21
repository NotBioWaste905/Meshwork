//! The frontier: tasks that can be picked up right now.

use std::collections::HashMap;

use crate::schemas::{Status, Task};

/// Tasks in `TODO` whose every dependency is `DONE`. With `unassigned_only`,
/// tasks that already have assignees are left out.
pub fn ready_tasks(tasks: Vec<Task>, unassigned_only: bool) -> Vec<Task> {
    let status: HashMap<&str, Status> = tasks.iter().map(|t| (t.id.as_str(), t.status)).collect();
    let ready: Vec<bool> = tasks
        .iter()
        .map(|t| {
            t.status == Status::Todo
                && (!unassigned_only || t.users.is_empty())
                && t.depends_on
                    .iter()
                    .all(|d| status.get(d.as_str()) == Some(&Status::Done))
        })
        .collect();
    tasks
        .into_iter()
        .zip(ready)
        .filter_map(|(t, ready)| ready.then_some(t))
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

    fn ids(tasks: Vec<Task>) -> Vec<String> {
        tasks.into_iter().map(|t| t.id).collect()
    }

    #[test]
    fn requires_todo_and_done_dependencies() {
        let tasks = vec![
            task("a", Status::Done, &[], &[]),
            task("b", Status::Todo, &["a"], &[]),
            task("c", Status::Todo, &["b"], &[]),
            task("d", Status::InProgress, &[], &[]),
            task("e", Status::Todo, &[], &[]),
            task("f", Status::Todo, &["a", "d"], &[]),
        ];
        assert_eq!(ids(ready_tasks(tasks, false)), vec!["b", "e"]);
    }

    #[test]
    fn can_skip_assigned() {
        let tasks = || {
            vec![
                task("a", Status::Todo, &[], &["alice"]),
                task("b", Status::Todo, &[], &[]),
            ]
        };
        assert_eq!(ids(ready_tasks(tasks(), true)), vec!["b"]);
        assert_eq!(ids(ready_tasks(tasks(), false)), vec!["a", "b"]);
    }

    #[test]
    fn unknown_dependency_is_not_done() {
        let tasks = vec![task("a", Status::Todo, &["ghost"], &[])];
        assert!(ready_tasks(tasks, false).is_empty());
    }
}
