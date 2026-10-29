//! Pure graph queries over a fetched task list.

use std::{cmp::Reverse, collections::HashMap};

use crate::model::{Status, Task};

/// Whose ready tasks to list, by assignees.
#[derive(Debug, Clone, Copy)]
pub enum Assigned<'a> {
    /// Assigned to anyone or nobody.
    Any,
    /// Nobody is assigned.
    Nobody,
    /// Nobody is assigned, or this actor id is among the assignees: what the actor may take.
    NobodyOr(&'a str),
}

impl Assigned<'_> {
    fn matches(self, t: &Task) -> bool {
        match self {
            Self::Any => true,
            Self::Nobody => t.users.is_empty(),
            Self::NobodyOr(me) => t.users.is_empty() || t.users.iter().any(|u| u == me),
        }
    }
}

/// Tasks an agent can pick up right now: `TODO` with every blocker `DONE`, highest
/// priority first (the backend's order is kept within a priority).
pub fn ready_tasks<'a>(tasks: &'a [Task], assigned: Assigned) -> Vec<&'a Task> {
    let by_id: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut ready: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == Status::Todo)
        .filter(|t| assigned.matches(t))
        .filter(|t| {
            t.depends_on.iter().all(|d| {
                by_id
                    .get(d.as_str())
                    .is_some_and(|b| b.status == Status::Done)
            })
        })
        .collect();
    ready.sort_by_key(|t| Reverse(t.priority));
    ready
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
    use crate::model::Priority;

    fn task(id: &str, status: Status, deps: &[&str], users: &[&str]) -> Task {
        Task {
            id: id.into(),
            name: id.into(),
            description: String::new(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            users: users.iter().map(|s| s.to_string()).collect(),
            reporter: None,
            created_at: None,
            updated_at: None,
            created_by: None,
            position: None,
            tags: vec![],
            status,
            priority: Priority::Normal,
            pr_url: String::new(),
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
        assert_eq!(ids(ready_tasks(&tasks, Assigned::Any)), vec!["b", "e"]);
    }

    #[test]
    fn ready_can_skip_assigned() {
        let tasks = vec![
            task("a", Status::Todo, &[], &["alice"]),
            task("b", Status::Todo, &[], &[]),
        ];
        assert_eq!(ids(ready_tasks(&tasks, Assigned::Nobody)), vec!["b"]);
        assert_eq!(ids(ready_tasks(&tasks, Assigned::Any)), vec!["a", "b"]);
    }

    #[test]
    fn ready_for_me_adds_my_tasks_not_others() {
        let tasks = vec![
            task("a", Status::Todo, &[], &["alice"]),
            task("b", Status::Todo, &[], &[]),
            task("c", Status::Todo, &[], &["me"]),
            task("d", Status::Todo, &[], &["alice", "me"]),
            task("e", Status::InProgress, &[], &["me"]),
        ];
        assert_eq!(
            ids(ready_tasks(&tasks, Assigned::NobodyOr("me"))),
            vec!["b", "c", "d"]
        );
    }

    #[test]
    fn ready_highest_priority_first() {
        let with = |id, priority| Task {
            priority,
            ..task(id, Status::Todo, &[], &[])
        };
        let tasks = vec![
            with("a", Priority::Low),
            with("b", Priority::Normal),
            with("c", Priority::High),
            with("d", Priority::Normal),
        ];
        assert_eq!(
            ids(ready_tasks(&tasks, Assigned::Any)),
            vec!["c", "b", "d", "a"]
        );
    }

    #[test]
    fn missing_blocker_is_not_ready() {
        let tasks = vec![task("a", Status::Todo, &["ghost"], &[])];
        assert!(ready_tasks(&tasks, Assigned::Any).is_empty());
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
