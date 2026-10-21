//! Cypher queries against FalkorDB.
//!
//! FalkorDB has no multi-query transactions: every query here is atomic on its
//! own, but a handler that runs several of them is not. Validation queries run
//! first so that a rejected request leaves no partial state behind.

use std::collections::BTreeSet;

use falkordb::{AsyncGraph, QueryResult, Row, RowStream};
use futures::StreamExt;

use crate::error::{db_err, AppError, Result};
use crate::schemas::{GraphInfo, Status, Task};

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
        .query("CREATE (:Graph {id: $id, name: $name, created_at: timestamp()})")
        .with_param("id", graph_id)
        .with_param("name", name)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

const GRAPH_RETURN: &str = "OPTIONAL MATCH (t:Task)-[:PART_OF]->(g) \
     RETURN g.id AS id, g.name AS name, g.created_at AS created_at, \
            count(t) AS task_count, \
            count(CASE WHEN t.status = 'DONE' THEN 1 END) AS done_count, \
            count(CASE WHEN t.status = 'BLOCKED' THEN 1 END) AS blocked_count";

fn row_to_graph(row: &Row) -> Result<GraphInfo> {
    Ok(GraphInfo {
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
        .query("MATCH (g:Graph {id: $graph_id}) SET g.name = $name")
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
        tags: row
            .try_get::<Option<Vec<String>>>("tags")
            .map_err(db_err)?
            .unwrap_or_default(),
        graph_id: graph_id.to_string(),
    })
}

const TASK_RETURN: &str = "OPTIONAL MATCH (t)-[:DEPENDS_ON]->(dep:Task) \
     OPTIONAL MATCH (t)-[:ASSIGNED_TO]->(u:User) \
     RETURN t.id AS id, t.name AS name, t.description AS description, \
            t.status AS status, t.tags AS tags, \
            collect(DISTINCT dep.id) AS dependencies, \
            collect(DISTINCT u.id) AS users \
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
    name: &str,
    description: &str,
    status: Status,
    tags: &[String],
) -> Result<()> {
    graph
        .query(
            "MATCH (g:Graph {id: $graph_id}) \
             CREATE (:Task {id: $id, name: $name, description: $description, \
                            status: $status, tags: $tags})-[:PART_OF]->(g)",
        )
        .with_param("graph_id", graph_id)
        .with_param("id", task_id)
        .with_param("name", name)
        .with_param("description", description)
        .with_param("status", status.as_str())
        .with_param("tags", tags.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

/// Updates the scalar properties that are `Some`.
pub async fn update_task_props(
    graph: &mut AsyncGraph,
    task_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    status: Option<Status>,
    tags: Option<&[String]>,
) -> Result<()> {
    // Only fixed fragments go into the query string; values are parameters.
    let sets: Vec<&str> = [
        name.map(|_| "t.name = $name"),
        description.map(|_| "t.description = $description"),
        status.map(|_| "t.status = $status"),
        tags.map(|_| "t.tags = $tags"),
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

/// Replaces the task's assignees. Users are created on first mention.
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
             UNWIND $users AS user_id \
             MERGE (u:User {id: user_id}) \
             MERGE (t)-[:ASSIGNED_TO]->(u)",
        )
        .with_param("id", task_id)
        .with_param("users", users.to_vec())
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
}

pub async fn delete_task(graph: &mut AsyncGraph, task_id: &str) -> Result<()> {
    graph
        .query("MATCH (t:Task {id: $id}) DETACH DELETE t")
        .with_param("id", task_id)
        .execute()
        .await
        .map_err(db_err)?;
    Ok(())
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
