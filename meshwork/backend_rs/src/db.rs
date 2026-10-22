//! Cypher queries against FalkorDB.
//!
//! FalkorDB has no multi-query transactions: every query here is atomic on its
//! own, but a handler that runs several of them is not. Validation queries run
//! first so that a rejected request leaves no partial state behind.

use std::collections::{BTreeMap, BTreeSet};

use falkordb::{AsyncGraph, QueryResult, Row, RowStream};
use futures::StreamExt;

use crate::error::{db_err, AppError, Result};
use crate::schemas::{
    Actor, ActorKind, Comment, GraphInfo, NewTask, Priority, Status, Task, TaskUpdate, TokenInfo,
};

async fn collect_rows(result: QueryResult<RowStream>) -> Result<Vec<Row>> {
    let mut data = result.data;
    let mut rows = Vec::new();
    while let Some(row) = data.next().await {
        rows.push(row.map_err(db_err)?);
    }
    Ok(rows)
}

fn first_count(rows: &[Row]) -> Result<i64> {
    match rows.first() {
        Some(row) => row.try_get("n").map_err(db_err),
        None => Ok(0),
    }
}

pub async fn create_graph(graph: &mut AsyncGraph, graph_id: &str, name: &str) -> Result<()> {
    graph
        .query(
            "CREATE (:Graph {id: $id, name: $name, created_at: timestamp(), \
                             updated_at: timestamp()})",
        )
        .with_param("id", graph_id)
        .with_param("name", name)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

const GRAPH_RETURN: &str = "OPTIONAL MATCH (g)-[:CREATED_BY]->(creator:Actor) \
     OPTIONAL MATCH (t:Task)-[:PART_OF]->(g) \
     RETURN g.id AS id, g.name AS name, g.created_at AS created_at, \
            g.updated_at AS own_updated_at, creator.id AS created_by, \
            max(t.updated_at) AS tasks_updated_at, \
            count(t) AS task_count, \
            count(CASE WHEN t.status = 'DONE' THEN 1 END) AS done_count, \
            count(CASE WHEN t.status = 'BLOCKED' THEN 1 END) AS blocked_count";

fn row_to_graph(row: &Row) -> Result<GraphInfo> {
    let created_at: Option<i64> = row.try_get("created_at").map_err(db_err)?;
    // The graph changes when it is renamed or any of its tasks does.
    let updated_at = [
        created_at,
        row.try_get::<Option<i64>>("own_updated_at")
            .map_err(db_err)?,
        row.try_get::<Option<i64>>("tasks_updated_at")
            .map_err(db_err)?,
    ]
    .into_iter()
    .flatten()
    .max();
    Ok(GraphInfo {
        created_at,
        updated_at,
        created_by: row.try_get("created_by").map_err(db_err)?,
        id: row.try_get("id").map_err(db_err)?,
        name: row
            .try_get::<Option<String>>("name")
            .map_err(db_err)?
            .unwrap_or_default(),
        task_count: row.try_get("task_count").map_err(db_err)?,
        done_count: row.try_get("done_count").map_err(db_err)?,
        blocked_count: row.try_get("blocked_count").map_err(db_err)?,
    })
}

/// All graphs, newest first; graphs without `created_at` (older ones) go last.
pub async fn list_graphs(graph: &mut AsyncGraph) -> Result<Vec<GraphInfo>> {
    let q = format!(
        "MATCH (g:Graph) {} ORDER BY coalesce(created_at, 0) DESC, id",
        GRAPH_RETURN
    );
    let result = graph.query(q).execute().await.map_err(db_err)?;
    collect_rows(result)
        .await?
        .iter()
        .map(row_to_graph)
        .collect()
}

pub async fn get_graph(graph: &mut AsyncGraph, graph_id: &str) -> Result<GraphInfo> {
    let q = format!("MATCH (g:Graph {{id: $graph_id}}) {}", GRAPH_RETURN);
    let result = graph
        .query(q)
        .with_param("graph_id", graph_id)
        .execute()
        .await
        .map_err(db_err)?;
    match collect_rows(result).await?.first() {
        Some(row) => row_to_graph(row),
        None => Err(AppError::NotFound(format!("Graph {} not found", graph_id))),
    }
}

pub async fn rename_graph(graph: &mut AsyncGraph, graph_id: &str, name: &str) -> Result<()> {
    graph
        .query("MATCH (g:Graph {id: $graph_id}) SET g.name = $name, g.updated_at = timestamp()")
        .with_param("graph_id", graph_id)
        .with_param("name", name)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn ensure_graph(graph: &mut AsyncGraph, graph_id: &str) -> Result<()> {
    let result = graph
        .query("MATCH (g:Graph {id: $graph_id}) RETURN count(g) AS n")
        .with_param("graph_id", graph_id)
        .execute()
        .await
        .map_err(db_err)?;
    if first_count(&collect_rows(result).await?)? == 0 {
        return Err(AppError::NotFound(format!("Graph {} not found", graph_id)));
    }
    Ok(())
}

/// Checks that every ID in `task_ids` is a task of `graph_id`.
pub async fn ensure_tasks_in_graph(
    graph: &mut AsyncGraph,
    graph_id: &str,
    task_ids: &[String],
) -> Result<()> {
    let unique: Vec<String> = task_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if unique.is_empty() {
        return Ok(());
    }
    let result = graph
        .query(
            "MATCH (t:Task)-[:PART_OF]->(:Graph {id: $graph_id}) \
             WHERE t.id IN $ids \
             RETURN count(t) AS n",
        )
        .with_param("graph_id", graph_id)
        .with_param("ids", unique.clone())
        .execute()
        .await
        .map_err(db_err)?;
    if first_count(&collect_rows(result).await?)? != unique.len() as i64 {
        return Err(AppError::NotFound(format!(
            "Some of the tasks {:?} are not in graph {}",
            unique, graph_id
        )));
    }
    Ok(())
}

fn row_to_task(row: &Row, graph_id: &str) -> Result<Task> {
    let status: String = row.try_get("status").map_err(db_err)?;
    let priority = match row.try_get::<Option<String>>("priority").map_err(db_err)? {
        Some(p) => Priority::parse(&p)
            .ok_or_else(|| AppError::Internal(format!("Bad priority '{}' in DB", p)))?,
        None => Priority::default(),
    };
    Ok(Task {
        id: row.try_get("id").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        description: row
            .try_get::<Option<String>>("description")
            .map_err(db_err)?
            .unwrap_or_default(),
        status: Status::parse(&status)
            .ok_or_else(|| AppError::Internal(format!("Bad status '{}' in DB", status)))?,
        depends_on: row.try_get("dependencies").map_err(db_err)?,
        users: row.try_get("users").map_err(db_err)?,
        reporter: row
            .try_get::<Vec<String>>("reporters")
            .map_err(db_err)?
            .into_iter()
            .next(),
        tags: row
            .try_get::<Option<Vec<String>>>("tags")
            .map_err(db_err)?
            .unwrap_or_default(),
        priority,
        pr_url: row
            .try_get::<Option<String>>("pr_url")
            .map_err(db_err)?
            .unwrap_or_default(),
        created_at: row.try_get("created_at").map_err(db_err)?,
        updated_at: row.try_get("updated_at").map_err(db_err)?,
        created_by: row
            .try_get::<Vec<String>>("creators")
            .map_err(db_err)?
            .into_iter()
            .next(),
        graph_id: graph_id.to_string(),
    })
}

const TASK_RETURN: &str = "OPTIONAL MATCH (t)-[:DEPENDS_ON]->(dep:Task) \
     OPTIONAL MATCH (t)-[:ASSIGNED_TO]->(u:Actor) \
     OPTIONAL MATCH (t)-[:REPORTED_BY]->(r:Actor) \
     OPTIONAL MATCH (t)-[:CREATED_BY]->(cb:Actor) \
     RETURN t.id AS id, t.name AS name, t.description AS description, \
            t.status AS status, t.tags AS tags, \
            t.priority AS priority, t.pr_url AS pr_url, \
            t.created_at AS created_at, t.updated_at AS updated_at, \
            collect(DISTINCT cb.id) AS creators, \
            collect(DISTINCT dep.id) AS dependencies, \
            collect(DISTINCT u.id) AS users, \
            collect(DISTINCT r.id) AS reporters \
     ORDER BY id";

pub async fn get_all_tasks(graph: &mut AsyncGraph, graph_id: &str) -> Result<Vec<Task>> {
    let q = format!(
        "MATCH (t:Task)-[:PART_OF]->(:Graph {{id: $graph_id}}) {}",
        TASK_RETURN
    );
    let result = graph
        .query(q)
        .with_param("graph_id", graph_id)
        .execute()
        .await
        .map_err(db_err)?;
    collect_rows(result)
        .await?
        .iter()
        .map(|row| row_to_task(row, graph_id))
        .collect()
}

pub async fn get_task(graph: &mut AsyncGraph, graph_id: &str, task_id: &str) -> Result<Task> {
    let q = format!(
        "MATCH (t:Task {{id: $task_id}})-[:PART_OF]->(:Graph {{id: $graph_id}}) {}",
        TASK_RETURN
    );
    let result = graph
        .query(q)
        .with_param("graph_id", graph_id)
        .with_param("task_id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    match collect_rows(result).await?.first() {
        Some(row) => row_to_task(row, graph_id),
        None => Err(AppError::NotFound(format!(
            "Task {} not found in graph {}",
            task_id, graph_id
        ))),
    }
}

pub async fn create_task(
    graph: &mut AsyncGraph,
    graph_id: &str,
    task_id: &str,
    task: &NewTask,
) -> Result<()> {
    graph
        .query(
            "MATCH (g:Graph {id: $graph_id}) \
             CREATE (:Task {id: $id, name: $name, description: $description, \
                            status: $status, tags: $tags, \
                            priority: $priority, pr_url: $pr_url, \
                            created_at: timestamp(), updated_at: timestamp()})-[:PART_OF]->(g)",
        )
        .with_param("graph_id", graph_id)
        .with_param("id", task_id)
        .with_param("name", task.name.as_str())
        .with_param("description", task.description.as_str())
        .with_param("status", task.status.as_str())
        .with_param("tags", task.tags.clone())
        .with_param("priority", task.priority.as_str())
        .with_param("pr_url", task.pr_url.as_str())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Updates the node's own properties that are `Some` in `update`; dependencies and
/// assignees (edges) are left to the caller.
pub async fn update_task_props(
    graph: &mut AsyncGraph,
    task_id: &str,
    update: &TaskUpdate,
) -> Result<()> {
    let name = update.name.as_deref();
    let description = update.description.as_deref();
    let status = update.status;
    let tags = update.tags.as_deref();
    let priority = update.priority;
    let pr_url = update.pr_url.as_deref();
    // Only fixed fragments go into the query string; values are parameters.
    let sets: Vec<&str> = [
        name.map(|_| "t.name = $name"),
        description.map(|_| "t.description = $description"),
        status.map(|_| "t.status = $status"),
        tags.map(|_| "t.tags = $tags"),
        priority.map(|_| "t.priority = $priority"),
        pr_url.map(|_| "t.pr_url = $pr_url"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if sets.is_empty() {
        return Ok(());
    }

    let q = format!("MATCH (t:Task {{id: $id}}) SET {}", sets.join(", "));
    let mut query = graph.query(q).with_param("id", task_id);
    if let Some(name) = name {
        query = query.with_param("name", name);
    }
    if let Some(description) = description {
        query = query.with_param("description", description);
    }
    if let Some(status) = status {
        query = query.with_param("status", status.as_str());
    }
    if let Some(tags) = tags {
        query = query.with_param("tags", tags.to_vec());
    }
    if let Some(priority) = priority {
        query = query.with_param("priority", priority.as_str());
    }
    if let Some(pr_url) = pr_url {
        query = query.with_param("pr_url", pr_url);
    }
    query.execute().await.map_err(db_err)?;
    Ok(())
}

/// Returns one of `depends_on` that already (transitively) depends on
/// `task_id`, i.e. a dependency whose edge would close a cycle.
pub async fn find_cycle(
    graph: &mut AsyncGraph,
    task_id: &str,
    depends_on: &[String],
) -> Result<Option<String>> {
    if depends_on.is_empty() {
        return Ok(None);
    }
    let result = graph
        .query(
            "UNWIND $deps AS dep_id \
             MATCH (:Task {id: dep_id})-[:DEPENDS_ON*1..]->(:Task {id: $id}) \
             RETURN dep_id AS id \
             LIMIT 1",
        )
        .with_param("id", task_id)
        .with_param("deps", depends_on.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    match collect_rows(result).await?.first() {
        Some(row) => Ok(Some(row.try_get("id").map_err(db_err)?)),
        None => Ok(None),
    }
}

/// Replaces the task's outgoing `DEPENDS_ON` edges with edges to `depends_on`.
pub async fn set_dependencies(
    graph: &mut AsyncGraph,
    task_id: &str,
    depends_on: &[String],
) -> Result<()> {
    graph
        .query("MATCH (t:Task {id: $id})-[r:DEPENDS_ON]->() DELETE r")
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    if depends_on.is_empty() {
        return Ok(());
    }
    graph
        .query(
            "MATCH (t:Task {id: $id}) \
             UNWIND $deps AS dep_id \
             MATCH (d:Task {id: dep_id}) \
             MERGE (t)-[:DEPENDS_ON]->(d)",
        )
        .with_param("id", task_id)
        .with_param("deps", depends_on.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Replaces the task's assignees. `users` are actor IDs that must exist (see
/// [`resolve_actors`]).
pub async fn set_users(graph: &mut AsyncGraph, task_id: &str, users: &[String]) -> Result<()> {
    graph
        .query("MATCH (t:Task {id: $id})-[r:ASSIGNED_TO]->() DELETE r")
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    if users.is_empty() {
        return Ok(());
    }
    graph
        .query(
            "MATCH (t:Task {id: $id}) \
             UNWIND $users AS actor_id \
             MATCH (a:Actor {id: actor_id}) \
             MERGE (t)-[:ASSIGNED_TO]->(a)",
        )
        .with_param("id", task_id)
        .with_param("users", users.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Claims the task for `actor_id` in one query, so two claimants cannot both win: it is set
/// `IN_PROGRESS` and assigned to the actor only if it is `TODO`, every task it depends on is
/// `DONE` and nobody else is assigned (being assigned already is fine). Returns whether it was
/// claimed; on `false` nothing changed.
pub async fn claim_task(
    graph: &mut AsyncGraph,
    graph_id: &str,
    task_id: &str,
    actor_id: &str,
) -> Result<bool> {
    let result = graph
        .query(
            "MATCH (t:Task {id: $id})-[:PART_OF]->(:Graph {id: $graph_id}) \
             WHERE t.status = $todo \
             OPTIONAL MATCH (t)-[:DEPENDS_ON]->(open:Task) WHERE open.status <> $done \
             WITH t, count(open) AS open_blockers \
             OPTIONAL MATCH (t)-[:ASSIGNED_TO]->(other:Actor) WHERE other.id <> $actor \
             WITH t, open_blockers, count(other) AS others \
             WHERE open_blockers = 0 AND others = 0 \
             MATCH (a:Actor {id: $actor}) \
             SET t.status = $in_progress, t.updated_at = timestamp() \
             MERGE (t)-[:ASSIGNED_TO]->(a) \
             RETURN count(t) AS n",
        )
        .with_param("graph_id", graph_id)
        .with_param("id", task_id)
        .with_param("actor", actor_id)
        .with_param("todo", Status::Todo.as_str())
        .with_param("done", Status::Done.as_str())
        .with_param("in_progress", Status::InProgress.as_str())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(first_count(&collect_rows(result).await?)? > 0)
}

/// Records who created a task or graph: `(x)-[:CREATED_BY]->(:Actor)`.
pub async fn set_created_by(
    graph: &mut AsyncGraph,
    label: Created,
    id: &str,
    actor_id: &str,
) -> Result<()> {
    // Only the fixed label goes into the query string.
    let q = format!(
        "MATCH (x:{} {{id: $id}}), (a:Actor {{id: $actor}}) CREATE (x)-[:CREATED_BY]->(a)",
        label.as_str()
    );
    graph
        .query(q)
        .with_param("id", id)
        .with_param("actor", actor_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// What `set_created_by` records the creator of.
#[derive(Clone, Copy)]
pub enum Created {
    Graph,
    Task,
}

impl Created {
    fn as_str(self) -> &'static str {
        match self {
            Created::Graph => "Graph",
            Created::Task => "Task",
        }
    }
}

/// Marks the task as changed now.
pub async fn touch_task(graph: &mut AsyncGraph, task_id: &str) -> Result<()> {
    graph
        .query("MATCH (t:Task {id: $id}) SET t.updated_at = timestamp()")
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Marks the graph as changed now, for changes no remaining task shows (a deleted task).
pub async fn touch_graph(graph: &mut AsyncGraph, graph_id: &str) -> Result<()> {
    graph
        .query("MATCH (g:Graph {id: $id}) SET g.updated_at = timestamp()")
        .with_param("id", graph_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Replaces the task's reporter; `None` removes it. The actor must exist (see
/// [`resolve_actors`]).
pub async fn set_reporter(
    graph: &mut AsyncGraph,
    task_id: &str,
    reporter: Option<&str>,
) -> Result<()> {
    graph
        .query("MATCH (t:Task {id: $id})-[r:REPORTED_BY]->() DELETE r")
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    if let Some(reporter) = reporter {
        graph
            .query(
                "MATCH (t:Task {id: $id}), (a:Actor {id: $reporter}) \
                 CREATE (t)-[:REPORTED_BY]->(a)",
            )
            .with_param("id", task_id)
            .with_param("reporter", reporter)
            .execute()
            .await
            .map_err(db_err)?;
    }
    Ok(())
}

/// Deletes the task with its edges and its comments.
pub async fn delete_task(graph: &mut AsyncGraph, task_id: &str) -> Result<()> {
    graph
        .query(
            "MATCH (t:Task {id: $id}) \
             OPTIONAL MATCH (c:Comment)-[:ON]->(t) \
             DETACH DELETE c, t",
        )
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Stores a comment as `(:Comment {id, body, created_at})-[:ON]->(:Task)` with
/// `(:Comment)-[:BY]->(:Actor)`.
pub async fn add_comment(
    graph: &mut AsyncGraph,
    task_id: &str,
    comment_id: &str,
    body: &str,
    author_id: &str,
) -> Result<()> {
    graph
        .query(
            "MATCH (t:Task {id: $task}), (a:Actor {id: $author}) \
             CREATE (c:Comment {id: $id, body: $body, created_at: timestamp()})-[:ON]->(t), \
                    (c)-[:BY]->(a)",
        )
        .with_param("task", task_id)
        .with_param("author", author_id)
        .with_param("id", comment_id)
        .with_param("body", body)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// The task's comments, oldest first.
pub async fn list_comments(graph: &mut AsyncGraph, task_id: &str) -> Result<Vec<Comment>> {
    let result = graph
        .query(
            "MATCH (c:Comment)-[:ON]->(:Task {id: $task}) \
             OPTIONAL MATCH (c)-[:BY]->(a:Actor) \
             RETURN c.id AS id, c.body AS body, c.created_at AS created_at, a.id AS author \
             ORDER BY created_at, id",
        )
        .with_param("task", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    collect_rows(result)
        .await?
        .iter()
        .map(|row| {
            Ok(Comment {
                id: row.try_get("id").map_err(db_err)?,
                body: row.try_get("body").map_err(db_err)?,
                author: row.try_get("author").map_err(db_err)?,
                created_at: row.try_get("created_at").map_err(db_err)?,
            })
        })
        .collect()
}

pub async fn connect(graph: &mut AsyncGraph, dependee: &str, dependency: &str) -> Result<()> {
    graph
        .query(
            "MATCH (a:Task {id: $dependee}), (b:Task {id: $dependency}) \
             MERGE (a)-[:DEPENDS_ON]->(b)",
        )
        .with_param("dependee", dependee)
        .with_param("dependency", dependency)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Removes the `DEPENDS_ON` edge; returns whether it existed.
pub async fn disconnect(graph: &mut AsyncGraph, dependee: &str, dependency: &str) -> Result<bool> {
    let result = graph
        .query(
            "MATCH (:Task {id: $dependee})-[r:DEPENDS_ON]->(:Task {id: $dependency}) \
             RETURN count(r) AS n",
        )
        .with_param("dependee", dependee)
        .with_param("dependency", dependency)
        .execute()
        .await
        .map_err(db_err)?;
    if first_count(&collect_rows(result).await?)? == 0 {
        return Ok(false);
    }
    graph
        .query("MATCH (:Task {id: $dependee})-[r:DEPENDS_ON]->(:Task {id: $dependency}) DELETE r")
        .with_param("dependee", dependee)
        .with_param("dependency", dependency)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(true)
}

/// Follows a `MATCH` that binds `a:Actor`; `count` is per actor.
const ACTOR_RETURN: &str = "OPTIONAL MATCH (t:Task)-[:ASSIGNED_TO]->(a) \
     RETURN a.id AS id, a.name AS name, a.kind AS kind, count(t) AS task_count";

fn row_to_actor(row: &Row) -> Result<Actor> {
    let kind: String = row.try_get("kind").map_err(db_err)?;
    Ok(Actor {
        id: row.try_get("id").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        kind: ActorKind::parse(&kind)
            .ok_or_else(|| AppError::Internal(format!("Bad actor kind '{}' in DB", kind)))?,
        task_count: row.try_get("task_count").map_err(db_err)?,
    })
}

/// All actors, by name.
pub async fn list_actors(graph: &mut AsyncGraph) -> Result<Vec<Actor>> {
    let q = format!(
        "MATCH (a:Actor) {} ORDER BY toLower(name), id",
        ACTOR_RETURN
    );
    let result = graph.query(q).execute().await.map_err(db_err)?;
    collect_rows(result)
        .await?
        .iter()
        .map(row_to_actor)
        .collect()
}

pub async fn get_actor(graph: &mut AsyncGraph, actor_id: &str) -> Result<Actor> {
    let q = format!("MATCH (a:Actor {{id: $id}}) {}", ACTOR_RETURN);
    let result = graph
        .query(q)
        .with_param("id", actor_id)
        .execute()
        .await
        .map_err(db_err)?;
    match collect_rows(result).await?.first() {
        Some(row) => row_to_actor(row),
        None => Err(AppError::NotFound(format!("Actor {} not found", actor_id))),
    }
}

/// Rejects `name` if another actor (not `actor_id`) already has it as its name
/// or ID: task assignees are resolved by either, so both must stay unambiguous.
pub async fn ensure_actor_name_free(
    graph: &mut AsyncGraph,
    name: &str,
    actor_id: &str,
) -> Result<()> {
    let result = graph
        .query(
            "MATCH (a:Actor) WHERE (a.name = $name OR a.id = $name) AND a.id <> $id \
             RETURN count(a) AS n",
        )
        .with_param("name", name)
        .with_param("id", actor_id)
        .execute()
        .await
        .map_err(db_err)?;
    if first_count(&collect_rows(result).await?)? > 0 {
        return Err(AppError::Conflict(format!(
            "An actor named '{}' already exists",
            name
        )));
    }
    Ok(())
}

pub async fn create_actor(
    graph: &mut AsyncGraph,
    actor_id: &str,
    name: &str,
    kind: ActorKind,
) -> Result<()> {
    graph
        .query("CREATE (:Actor {id: $id, name: $name, kind: $kind})")
        .with_param("id", actor_id)
        .with_param("name", name)
        .with_param("kind", kind.as_str())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Updates the properties that are `Some`.
pub async fn update_actor(
    graph: &mut AsyncGraph,
    actor_id: &str,
    name: Option<&str>,
    kind: Option<ActorKind>,
) -> Result<()> {
    let sets: Vec<&str> = [
        name.map(|_| "a.name = $name"),
        kind.map(|_| "a.kind = $kind"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if sets.is_empty() {
        return Ok(());
    }
    let q = format!("MATCH (a:Actor {{id: $id}}) SET {}", sets.join(", "));
    let mut query = graph.query(q).with_param("id", actor_id);
    if let Some(name) = name {
        query = query.with_param("name", name);
    }
    if let Some(kind) = kind {
        query = query.with_param("kind", kind.as_str());
    }
    query.execute().await.map_err(db_err)?;
    Ok(())
}

/// Deletes the actor together with its `ASSIGNED_TO` and `REPORTED_BY` edges (the tasks it
/// reported are left without a reporter) and its tokens. Its comments stay, without an author,
/// and what it created without a creator.
pub async fn delete_actor(graph: &mut AsyncGraph, actor_id: &str) -> Result<()> {
    graph
        .query(
            "MATCH (a:Actor {id: $id}) \
             OPTIONAL MATCH (k:Token)-[:OF]->(a) \
             DETACH DELETE k, a",
        )
        .with_param("id", actor_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Moves the tasks assigned to or reported by `from`, what it created, its comments and its
/// tokens over to `into`, then deletes `from`. A task assigned to both keeps one edge; a token of the duplicate now acts as `into`.
/// If interrupted between the queries, `from` is still there and the merge can simply be
/// repeated.
pub async fn merge_actor(graph: &mut AsyncGraph, from: &str, into: &str) -> Result<()> {
    graph
        .query(
            "MATCH (t:Task)-[:ASSIGNED_TO]->(:Actor {id: $from}) \
             MATCH (d:Actor {id: $into}) \
             MERGE (t)-[:ASSIGNED_TO]->(d)",
        )
        .with_param("from", from)
        .with_param("into", into)
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query(
            "MATCH (t:Task)-[r:REPORTED_BY]->(:Actor {id: $from}) \
             MATCH (d:Actor {id: $into}) \
             CREATE (t)-[:REPORTED_BY]->(d) \
             DELETE r",
        )
        .with_param("from", from)
        .with_param("into", into)
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query(
            "MATCH (c:Comment)-[r:BY]->(:Actor {id: $from}) \
             MATCH (d:Actor {id: $into}) \
             CREATE (c)-[:BY]->(d) \
             DELETE r",
        )
        .with_param("from", from)
        .with_param("into", into)
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query(
            "MATCH (x)-[r:CREATED_BY]->(:Actor {id: $from}) \
             MATCH (d:Actor {id: $into}) \
             CREATE (x)-[:CREATED_BY]->(d) \
             DELETE r",
        )
        .with_param("from", from)
        .with_param("into", into)
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query(
            "MATCH (k:Token)-[r:OF]->(:Actor {id: $from}) \
             MATCH (d:Actor {id: $into}) \
             CREATE (k)-[:OF]->(d) \
             DELETE r",
        )
        .with_param("from", from)
        .with_param("into", into)
        .execute()
        .await
        .map_err(db_err)?;
    delete_actor(graph, from).await
}

/// Stores an API token as `(:Token {id, hash, label, created_at})-[:OF]->(:Actor)`. Only the
/// hash is stored; the token itself is shown once, by whoever issued it.
pub async fn create_token(
    graph: &mut AsyncGraph,
    token_id: &str,
    hash: &str,
    actor_id: &str,
    label: &str,
) -> Result<()> {
    graph
        .query(
            "MATCH (a:Actor {id: $actor}) \
             CREATE (:Token {id: $id, hash: $hash, label: $label, created_at: timestamp()})-[:OF]->(a)",
        )
        .with_param("actor", actor_id)
        .with_param("id", token_id)
        .with_param("hash", hash)
        .with_param("label", label)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// The actor a token hash belongs to, if the token exists.
pub async fn actor_by_token_hash(graph: &mut AsyncGraph, hash: &str) -> Result<Option<Actor>> {
    let q = format!(
        "MATCH (:Token {{hash: $hash}})-[:OF]->(a:Actor) {}",
        ACTOR_RETURN
    );
    let result = graph
        .query(q)
        .with_param("hash", hash)
        .execute()
        .await
        .map_err(db_err)?;
    collect_rows(result)
        .await?
        .first()
        .map(row_to_actor)
        .transpose()
}

/// ID of the token with this hash, if any.
pub async fn token_id_by_hash(graph: &mut AsyncGraph, hash: &str) -> Result<Option<String>> {
    let result = graph
        .query("MATCH (k:Token {hash: $hash}) RETURN k.id AS id")
        .with_param("hash", hash)
        .execute()
        .await
        .map_err(db_err)?;
    collect_rows(result)
        .await?
        .first()
        .map(|row| row.try_get("id").map_err(db_err))
        .transpose()
}

/// Tokens of one actor, or of everyone; never the hashes.
pub async fn list_tokens(graph: &mut AsyncGraph, actor_id: Option<&str>) -> Result<Vec<TokenInfo>> {
    let q = format!(
        "MATCH (k:Token)-[:OF]->(a:Actor) {} \
         RETURN k.id AS id, k.label AS label, k.created_at AS created_at, \
                a.id AS actor_id, a.name AS actor_name \
         ORDER BY toLower(actor_name), created_at",
        if actor_id.is_some() {
            "WHERE a.id = $actor"
        } else {
            ""
        }
    );
    let mut query = graph.query(q);
    if let Some(actor_id) = actor_id {
        query = query.with_param("actor", actor_id);
    }
    let result = query.execute().await.map_err(db_err)?;
    collect_rows(result)
        .await?
        .iter()
        .map(|row| {
            Ok(TokenInfo {
                id: row.try_get("id").map_err(db_err)?,
                label: row.try_get("label").map_err(db_err)?,
                created_at: row.try_get("created_at").map_err(db_err)?,
                actor_id: row.try_get("actor_id").map_err(db_err)?,
                actor_name: row.try_get("actor_name").map_err(db_err)?,
            })
        })
        .collect()
}

/// Deletes a token; returns whether it existed.
pub async fn revoke_token(graph: &mut AsyncGraph, token_id: &str) -> Result<bool> {
    let result = graph
        .query("MATCH (k:Token {id: $id}) RETURN count(k) AS n")
        .with_param("id", token_id)
        .execute()
        .await
        .map_err(db_err)?;
    if first_count(&collect_rows(result).await?)? == 0 {
        return Ok(false);
    }
    graph
        .query("MATCH (k:Token {id: $id}) DETACH DELETE k")
        .with_param("id", token_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(true)
}

/// Maps each reference — an actor ID or, failing that, an actor name — to the
/// actor's ID, dropping duplicates and keeping the order. Unknown references
/// are a 404.
pub async fn resolve_actors(graph: &mut AsyncGraph, refs: &[String]) -> Result<Vec<String>> {
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    let result = graph
        .query(
            "MATCH (a:Actor) WHERE a.id IN $refs OR a.name IN $refs \
             RETURN a.id AS id, a.name AS name",
        )
        .with_param("refs", refs.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    let mut by_id = BTreeSet::new();
    let mut by_name = BTreeMap::new();
    for row in collect_rows(result).await? {
        let id: String = row.try_get("id").map_err(db_err)?;
        let name: String = row.try_get("name").map_err(db_err)?;
        by_name.insert(name, id.clone());
        by_id.insert(id);
    }

    let mut ids: Vec<String> = Vec::new();
    let mut unknown: Vec<&str> = Vec::new();
    for r in refs {
        let id = if by_id.contains(r) {
            Some(r.clone())
        } else {
            by_name.get(r).cloned()
        };
        match id {
            Some(id) if !ids.contains(&id) => ids.push(id),
            Some(_) => {}
            None => unknown.push(r),
        }
    }
    if !unknown.is_empty() {
        return Err(AppError::NotFound(format!(
            "Unknown actors {:?}; create them with POST /v0/actors",
            unknown
        )));
    }
    Ok(ids)
}

/// Free-text assignees that were AI agents by project convention.
const KNOWN_AGENT_NAMES: [&str; 1] = ["claude"];

/// Turns the old free-text assignees, `(:User {id: <text>})` nodes, into
/// actors: each becomes `(:Actor {id: <text>, name: <text>})`, so `Task.users`
/// returns the same strings as before, and its `ASSIGNED_TO` edges move over.
/// `kind` is `agent` for [`KNOWN_AGENT_NAMES`], `human` otherwise. Idempotent,
/// and safe to rerun after an interruption: every step only acts on what is
/// still left. Returns the number of `:User` nodes migrated.
pub async fn migrate_users_to_actors(graph: &mut AsyncGraph) -> Result<i64> {
    let result = graph
        .query("MATCH (u:User) RETURN count(u) AS n")
        .execute()
        .await
        .map_err(db_err)?;
    let users = first_count(&collect_rows(result).await?)?;
    if users == 0 {
        return Ok(0);
    }
    let agents: Vec<String> = KNOWN_AGENT_NAMES.iter().map(|s| s.to_string()).collect();
    graph
        .query(
            "MATCH (u:User) \
             MERGE (a:Actor {id: u.id}) \
             ON CREATE SET a.name = u.id, \
                           a.kind = CASE WHEN toLower(u.id) IN $agents \
                                         THEN 'agent' ELSE 'human' END",
        )
        .with_param("agents", agents)
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query(
            "MATCH (t:Task)-[r:ASSIGNED_TO]->(u:User) \
             MATCH (a:Actor {id: u.id}) \
             MERGE (t)-[:ASSIGNED_TO]->(a) \
             DELETE r",
        )
        .execute()
        .await
        .map_err(db_err)?;
    graph
        .query("MATCH (u:User) DETACH DELETE u")
        .execute()
        .await
        .map_err(db_err)?;
    Ok(users)
}
