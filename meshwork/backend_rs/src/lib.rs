// The falkordb query futures are deeply nested; rustc >= 1.9x needs more than the default 128.
#![recursion_limit = "256"]

use axum::{
    body::Bytes,
    extract::{Extension, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use falkordb::AsyncGraph;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use uuid::Uuid;

pub mod auth;
pub mod connection;
mod db;
pub mod error;
pub mod events;
pub mod frontier;
pub mod schemas;

use error::{AppError, Result};
use events::Change;
use schemas::{
    Actor, ActorUpdate, Comment, ConnectNodesRequest, CreateGraphResponse, Event, GraphInfo,
    GraphUpdate, IssuedToken, LayoutUpdate, MergeActors, MessageResponse, NewActor, NewComment,
    NewGraph, NewSpace, NewTask, NewToken, OwnToken, Space, SpaceUpdate, Status, Task, TaskUpdate,
    DEFAULT_SPACE_ID,
};

#[derive(Clone)]
pub struct AppState {
    pub graph: AsyncGraph,
}

/// Builds the CORS layer from a comma-separated list of origins, or `*`.
pub fn cors_layer(allowed_origins: &str) -> CorsLayer {
    let layer = CorsLayer::new().allow_methods(Any).allow_headers(Any);
    if allowed_origins.trim() == "*" {
        return layer.allow_origin(Any);
    }
    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .filter_map(|o| o.parse().ok())
        .collect();
    layer.allow_origin(origins)
}

pub fn app(state: AppState, cors: CorsLayer) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v0/create_graph", post(create_graph))
        .route("/v0/graphs", get(list_graphs))
        .route("/v0/spaces", get(list_spaces).post(add_space))
        .route(
            "/v0/spaces/:space_id",
            get(get_space).put(edit_space).delete(delete_space),
        )
        .route("/v0/spaces/:space_id/graphs", get(list_space_graphs))
        .route("/v0/me", get(me))
        .route("/v0/me/tokens", get(list_own_tokens).post(issue_own_token))
        .route("/v0/me/tokens/:token_id", delete(revoke_own_token))
        .route("/v0/actors", get(list_actors).post(add_actor))
        .route(
            "/v0/actors/:actor_id",
            get(get_actor).put(edit_actor).delete(delete_actor),
        )
        .route("/v0/actors/:actor_id/merge", post(merge_actor))
        .route("/v0/:graph_id", get(get_graph).put(edit_graph))
        .route("/v0/:graph_id/task/add/", post(add_task))
        .route("/v0/:graph_id/all_tasks", get(get_all_tasks))
        .route("/v0/:graph_id/ready_tasks", get(get_ready_tasks))
        .route(
            "/v0/:graph_id/task/:task_id",
            get(get_task).put(edit_task).delete(delete_task),
        )
        .route("/v0/:graph_id/task/:task_id/claim", post(claim_task))
        .route("/v0/:graph_id/task/:task_id/events", get(list_events))
        .route(
            "/v0/:graph_id/task/:task_id/comments",
            get(list_comments).post(add_comment),
        )
        .route("/v0/:graph_id/layout", axum::routing::put(save_layout))
        .route("/v0/:graph_id/connect_nodes/", post(connect_nodes))
        .route("/v0/:graph_id/disconnect_nodes/", post(disconnect_nodes))
        // The last layer added runs first: the connection check, then the token lookup.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::identify,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            connection::ensure_connection_layer,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn root() -> &'static str {
    "Meshwork backend"
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "healthy", "service": "meshwork-backend" }))
}

/// The body is optional: no body (or an empty one) creates a graph without a name, so
/// clients written before graphs had names keep working.
async fn create_graph(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
    body: Bytes,
) -> Result<Json<CreateGraphResponse>> {
    let new_graph: NewGraph = if body.iter().all(u8::is_ascii_whitespace) {
        NewGraph::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|e| AppError::BadRequest(format!("Invalid graph body: {}", e)))?
    };
    let name = new_graph.name.trim().to_string();
    let mut graph = state.graph.clone();
    let space = match new_graph.space.as_deref().filter(|s| !s.is_empty()) {
        Some(space) if space != DEFAULT_SPACE_ID => {
            db::get_space(&mut graph, space).await?;
            space.to_string()
        }
        _ => DEFAULT_SPACE_ID.to_string(),
    };
    let id = Uuid::new_v4().to_string();
    info!("Creating new graph with ID: {} in space {}", id, space);
    db::create_graph(&mut graph, &id, &name, &space).await?;
    if let Some(actor) = &caller.0 {
        db::set_created_by(&mut graph, db::Created::Graph, &id, &actor.id).await?;
    }
    Ok(Json(CreateGraphResponse { id, name, space }))
}

#[derive(serde::Deserialize)]
struct ListGraphsQuery {
    space: Option<String>,
}

/// All graphs, or with `?space=<id>` those of one space (404 if there is no such space).
async fn list_graphs(
    State(state): State<AppState>,
    Query(query): Query<ListGraphsQuery>,
) -> Result<Json<Vec<GraphInfo>>> {
    let mut graph = state.graph.clone();
    if let Some(space) = &query.space {
        db::get_space(&mut graph, space).await?;
    }
    Ok(Json(
        db::list_graphs(&mut graph, query.space.as_deref()).await?,
    ))
}

async fn list_space_graphs(
    State(state): State<AppState>,
    Path(space_id): Path<String>,
) -> Result<Json<Vec<GraphInfo>>> {
    let mut graph = state.graph.clone();
    db::get_space(&mut graph, &space_id).await?;
    Ok(Json(db::list_graphs(&mut graph, Some(&space_id)).await?))
}

async fn list_spaces(State(state): State<AppState>) -> Result<Json<Vec<Space>>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::list_spaces(&mut graph).await?))
}

async fn get_space(
    State(state): State<AppState>,
    Path(space_id): Path<String>,
) -> Result<Json<Space>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::get_space(&mut graph, &space_id).await?))
}

async fn add_space(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
    Json(space): Json<NewSpace>,
) -> Result<(StatusCode, Json<Space>)> {
    let name = space.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "Space name cannot be empty".to_string(),
        ));
    }
    let id = Uuid::new_v4().to_string();
    info!("Creating space '{}' ({})", id, name);
    let mut graph = state.graph.clone();
    db::create_space(&mut graph, &id, name).await?;
    if let Some(actor) = &caller.0 {
        db::set_created_by(&mut graph, db::Created::Space, &id, &actor.id).await?;
    }
    Ok((
        StatusCode::CREATED,
        Json(db::get_space(&mut graph, &id).await?),
    ))
}

async fn edit_space(
    State(state): State<AppState>,
    Path(space_id): Path<String>,
    Json(update): Json<SpaceUpdate>,
) -> Result<Json<Space>> {
    let name = update.name.as_deref().map(str::trim);
    if name == Some("") {
        return Err(AppError::BadRequest(
            "Space name cannot be empty".to_string(),
        ));
    }
    let mut graph = state.graph.clone();
    db::get_space(&mut graph, &space_id).await?;
    if let Some(name) = name {
        info!("Renaming space '{}'", space_id);
        db::rename_space(&mut graph, &space_id, name).await?;
    }
    Ok(Json(db::get_space(&mut graph, &space_id).await?))
}

/// Deletes an empty space; 409 while it has graphs, and always for the default space, which
/// graphs created without a space go to.
async fn delete_space(
    State(state): State<AppState>,
    Path(space_id): Path<String>,
) -> Result<Json<MessageResponse>> {
    let mut graph = state.graph.clone();
    let space = db::get_space(&mut graph, &space_id).await?;
    if space_id == DEFAULT_SPACE_ID {
        return Err(AppError::Conflict(
            "The default space cannot be deleted".to_string(),
        ));
    }
    // Checked again inside the delete query, in case a graph moved in meanwhile.
    if space.graph_count > 0 || !db::delete_empty_space(&mut graph, &space_id).await? {
        return Err(AppError::Conflict(format!(
            "Space '{}' still has graphs; move or delete them first",
            space.name
        )));
    }
    info!("Deleted space '{}' ({})", space_id, space.name);
    Ok(Json(MessageResponse {
        message: format!("Space {} deleted", space_id),
    }))
}

async fn get_graph(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
) -> Result<Json<GraphInfo>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::get_graph(&mut graph, &graph_id).await?))
}

async fn edit_graph(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Json(update): Json<GraphUpdate>,
) -> Result<Json<GraphInfo>> {
    info!("Editing graph '{}'", graph_id);
    if matches!(&update.name, Some(name) if name.trim().is_empty()) {
        return Err(AppError::BadRequest(
            "Graph name cannot be empty".to_string(),
        ));
    }
    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    if let Some(space) = &update.space {
        db::get_space(&mut graph, space).await?;
    }
    if let Some(name) = &update.name {
        db::rename_graph(&mut graph, &graph_id, name.trim()).await?;
    }
    if let Some(space) = &update.space {
        info!("Moving graph '{}' to space '{}'", graph_id, space);
        db::move_graph(&mut graph, &graph_id, space).await?;
    }
    Ok(Json(db::get_graph(&mut graph, &graph_id).await?))
}

/// The actor the request's bearer token belongs to; 401 without a token.
async fn me(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
) -> Result<Json<Actor>> {
    let actor = caller
        .0
        .ok_or_else(|| AppError::Unauthorized("No bearer token".to_string()))?;
    let mut graph = state.graph.clone();
    // fresh, so task_count is current
    Ok(Json(db::get_actor(&mut graph, &actor.id).await?))
}

/// The request's actor, or 401: the token endpoints act on the caller's own tokens only, so
/// nobody can get a token for an identity they do not already hold.
fn require_actor(caller: auth::Caller) -> Result<Actor> {
    caller
        .0
        .ok_or_else(|| AppError::Unauthorized("Managing tokens needs a bearer token".to_string()))
}

const MAX_TOKEN_LABEL_CHARS: usize = 100;

async fn list_own_tokens(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
    headers: HeaderMap,
) -> Result<Json<Vec<OwnToken>>> {
    let actor = require_actor(caller)?;
    let mut graph = state.graph.clone();
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    let current = match auth::bearer_token(presented) {
        Some(token) => db::token_id_by_hash(&mut graph, &auth::hash_token(token)).await?,
        None => None,
    };
    let tokens = db::list_tokens(&mut graph, Some(&actor.id)).await?;
    Ok(Json(
        tokens
            .into_iter()
            .map(|info| OwnToken {
                current: Some(&info.id) == current.as_ref(),
                info,
            })
            .collect(),
    ))
}

/// Issues the caller another token, e.g. for a second device or an agent acting as them.
async fn issue_own_token(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
    Json(request): Json<NewToken>,
) -> Result<(StatusCode, Json<IssuedToken>)> {
    let actor = require_actor(caller)?;
    let label = request.label.trim();
    if label.chars().count() > MAX_TOKEN_LABEL_CHARS {
        return Err(AppError::BadRequest(format!(
            "Token label is longer than {} characters",
            MAX_TOKEN_LABEL_CHARS
        )));
    }
    let mut graph = state.graph.clone();
    let (token, id, _) = auth::issue_token(&mut graph, &actor.id, label).await?;
    info!("Actor '{}' issued themselves token {}", actor.name, id);
    let info = db::list_tokens(&mut graph, Some(&actor.id))
        .await?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::Internal(format!("Token {} vanished", id)))?;
    Ok((StatusCode::CREATED, Json(IssuedToken { token, info })))
}

/// Revokes one of the caller's tokens; someone else's token is a 404, as if it did not exist.
async fn revoke_own_token(
    State(state): State<AppState>,
    Extension(caller): Extension<auth::Caller>,
    Path(token_id): Path<String>,
) -> Result<Json<MessageResponse>> {
    let actor = require_actor(caller)?;
    let mut graph = state.graph.clone();
    let own = db::list_tokens(&mut graph, Some(&actor.id)).await?;
    if !own.iter().any(|t| t.id == token_id) {
        return Err(AppError::NotFound(format!("Token {} not found", token_id)));
    }
    auth::revoke_token(&mut graph, &token_id).await?;
    info!("Actor '{}' revoked their token {}", actor.name, token_id);
    Ok(Json(MessageResponse {
        message: format!("Token {} revoked", token_id),
    }))
}

async fn list_actors(State(state): State<AppState>) -> Result<Json<Vec<Actor>>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::list_actors(&mut graph).await?))
}

async fn get_actor(
    State(state): State<AppState>,
    Path(actor_id): Path<String>,
) -> Result<Json<Actor>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::get_actor(&mut graph, &actor_id).await?))
}

async fn add_actor(
    State(state): State<AppState>,
    Json(actor): Json<NewActor>,
) -> Result<(StatusCode, Json<Actor>)> {
    let name = actor.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "Actor name cannot be empty".to_string(),
        ));
    }
    let id = Uuid::new_v4().to_string();
    let mut graph = state.graph.clone();
    db::ensure_actor_name_free(&mut graph, name, &id).await?;
    info!("Adding actor '{}' ({})", id, name);
    db::create_actor(&mut graph, &id, name, actor.kind).await?;
    Ok((
        StatusCode::CREATED,
        Json(db::get_actor(&mut graph, &id).await?),
    ))
}

async fn edit_actor(
    State(state): State<AppState>,
    Path(actor_id): Path<String>,
    Json(update): Json<ActorUpdate>,
) -> Result<Json<Actor>> {
    let name = update.name.as_deref().map(str::trim);
    if name == Some("") {
        return Err(AppError::BadRequest(
            "Actor name cannot be empty".to_string(),
        ));
    }
    let mut graph = state.graph.clone();
    db::get_actor(&mut graph, &actor_id).await?;
    if let Some(name) = name {
        db::ensure_actor_name_free(&mut graph, name, &actor_id).await?;
    }
    info!("Editing actor '{}'", actor_id);
    db::update_actor(&mut graph, &actor_id, name, update.kind).await?;
    Ok(Json(db::get_actor(&mut graph, &actor_id).await?))
}

#[derive(serde::Deserialize)]
struct DeleteActorQuery {
    #[serde(default)]
    unassign: bool,
}

/// Deletes an actor. An actor still assigned to tasks is a 409 unless `?unassign=true`, which
/// drops those assignments; to hand the tasks to someone else, merge instead.
async fn delete_actor(
    State(state): State<AppState>,
    Path(actor_id): Path<String>,
    Query(query): Query<DeleteActorQuery>,
) -> Result<Json<MessageResponse>> {
    let mut graph = state.graph.clone();
    let actor = db::get_actor(&mut graph, &actor_id).await?;
    if actor.task_count > 0 && !query.unassign {
        return Err(AppError::Conflict(format!(
            "Actor '{}' is assigned to {} task(s); pass ?unassign=true or merge the actor",
            actor.name, actor.task_count
        )));
    }
    info!("Deleting actor '{}' ({})", actor_id, actor.name);
    db::delete_actor(&mut graph, &actor_id).await?;
    Ok(Json(MessageResponse {
        message: format!("Actor {} deleted", actor_id),
    }))
}

/// Merges a duplicate into another actor: its tasks move over and it is deleted. Returns the
/// actor that stays.
async fn merge_actor(
    State(state): State<AppState>,
    Path(actor_id): Path<String>,
    Json(request): Json<MergeActors>,
) -> Result<Json<Actor>> {
    let mut graph = state.graph.clone();
    db::get_actor(&mut graph, &actor_id).await?;
    let into = db::resolve_actors(&mut graph, std::slice::from_ref(&request.into))
        .await?
        .remove(0);
    if into == actor_id {
        return Err(AppError::BadRequest(
            "An actor cannot be merged into itself".to_string(),
        ));
    }
    info!("Merging actor '{}' into '{}'", actor_id, into);
    db::merge_actor(&mut graph, &actor_id, &into).await?;
    Ok(Json(db::get_actor(&mut graph, &into).await?))
}

async fn add_task(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Extension(caller): Extension<auth::Caller>,
    Json(task): Json<NewTask>,
) -> Result<(StatusCode, Json<Task>)> {
    if task.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Task name cannot be empty".to_string(),
        ));
    }
    validate_pr_url(&task.pr_url)?;

    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    db::ensure_tasks_in_graph(&mut graph, &graph_id, &task.depends_on).await?;
    let users = db::resolve_actors(&mut graph, &task.users).await?;
    // The author reports the task unless someone else is named.
    let reporter = match task.reporter.as_deref().filter(|r| !r.is_empty()) {
        Some(r) => Some(resolve_actor(&mut graph, r).await?),
        None => caller.0.as_ref().map(|actor| actor.id.clone()),
    };

    let id = Uuid::new_v4().to_string();
    info!("Adding task '{}' to graph '{}'", id, graph_id);
    db::create_task(&mut graph, &graph_id, &id, &task).await?;
    db::set_dependencies(&mut graph, &id, &task.depends_on).await?;
    db::set_users(&mut graph, &id, &users).await?;
    db::set_reporter(&mut graph, &id, reporter.as_deref()).await?;
    if let Some(actor) = &caller.0 {
        db::set_created_by(&mut graph, db::Created::Task, &id, &actor.id).await?;
    }
    db::add_events(&mut graph, &id, actor_id(&caller), &[Change::created()]).await?;

    let created = db::get_task(&mut graph, &graph_id, &id).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn get_all_tasks(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
) -> Result<Json<Vec<Task>>> {
    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    Ok(Json(db::get_all_tasks(&mut graph, &graph_id).await?))
}

#[derive(serde::Deserialize)]
struct ReadyTasksQuery {
    #[serde(default)]
    unassigned_only: bool,
}

/// Tasks in `TODO` whose dependencies are all `DONE`.
async fn get_ready_tasks(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Query(query): Query<ReadyTasksQuery>,
) -> Result<Json<Vec<Task>>> {
    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    let tasks = db::get_all_tasks(&mut graph, &graph_id).await?;
    Ok(Json(frontier::ready_tasks(tasks, query.unassigned_only)))
}

async fn get_task(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
) -> Result<Json<Task>> {
    let mut graph = state.graph.clone();
    Ok(Json(db::get_task(&mut graph, &graph_id, &task_id).await?))
}

async fn edit_task(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
    Extension(caller): Extension<auth::Caller>,
    Json(update): Json<TaskUpdate>,
) -> Result<Json<Task>> {
    info!("Editing task '{}' in graph '{}'", task_id, graph_id);
    if matches!(&update.name, Some(name) if name.trim().is_empty()) {
        return Err(AppError::BadRequest(
            "Task name cannot be empty".to_string(),
        ));
    }
    if let Some(pr_url) = &update.pr_url {
        validate_pr_url(pr_url)?;
    }

    let mut graph = state.graph.clone();
    let before = db::get_task(&mut graph, &graph_id, &task_id).await?;
    if let Some(deps) = &update.depends_on {
        if deps.contains(&task_id) {
            return Err(AppError::BadRequest(
                "A task cannot depend on itself".to_string(),
            ));
        }
        db::ensure_tasks_in_graph(&mut graph, &graph_id, deps).await?;
        ensure_no_cycle(&mut graph, &task_id, deps).await?;
    }
    let users = match &update.users {
        Some(refs) => Some(db::resolve_actors(&mut graph, refs).await?),
        None => None,
    };
    // Some(None): remove the reporter.
    let reporter = match update.reporter.as_deref() {
        Some("") => Some(None),
        Some(r) => Some(Some(resolve_actor(&mut graph, r).await?)),
        None => None,
    };

    db::update_task_props(&mut graph, &task_id, &update).await?;
    if let Some(deps) = &update.depends_on {
        db::set_dependencies(&mut graph, &task_id, deps).await?;
    }
    if let Some(users) = &users {
        db::set_users(&mut graph, &task_id, users).await?;
    }
    if let Some(reporter) = &reporter {
        db::set_reporter(&mut graph, &task_id, reporter.as_deref()).await?;
    }
    db::touch_task(&mut graph, &task_id).await?;

    let after = db::get_task(&mut graph, &graph_id, &task_id).await?;
    let mut changes = events::diff(&before, &after);
    changes.extend(
        after
            .depends_on
            .iter()
            .filter(|d| !before.depends_on.contains(d))
            .map(|d| Change::blocker(d, true)),
    );
    changes.extend(
        before
            .depends_on
            .iter()
            .filter(|d| !after.depends_on.contains(d))
            .map(|d| Change::blocker(d, false)),
    );
    db::add_events(&mut graph, &task_id, actor_id(&caller), &changes).await?;
    Ok(Json(after))
}

/// The ID of the request's actor, if it has a token.
fn actor_id(caller: &auth::Caller) -> Option<&str> {
    caller.0.as_ref().map(|actor| actor.id.as_str())
}

async fn list_events(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<Event>>> {
    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    Ok(Json(db::list_events(&mut graph, &task_id).await?))
}

/// One actor by ID or name; 404 if there is none.
async fn resolve_actor(graph: &mut AsyncGraph, actor: &str) -> Result<String> {
    Ok(db::resolve_actors(graph, &[actor.to_string()])
        .await?
        .remove(0))
}

/// A PR link is shown as a clickable link, so only `http(s)` URLs (or nothing) are accepted.
fn validate_pr_url(pr_url: &str) -> Result<()> {
    let lower = pr_url.to_ascii_lowercase();
    if pr_url.is_empty() || lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "PR link must be an http(s) URL or empty, got '{}'",
            pr_url
        )))
    }
}

/// Takes a task for the caller: `IN_PROGRESS` and assigned to them, atomically, if it is ready
/// (`TODO`, every blocker `DONE`) and nobody else has it. 401 without a token, 409 with the reason
/// when the task cannot be claimed (someone was faster, it is blocked, not `TODO`).
async fn claim_task(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
    Extension(caller): Extension<auth::Caller>,
) -> Result<Json<Task>> {
    let actor = caller
        .0
        .ok_or_else(|| AppError::Unauthorized("Claiming needs a bearer token".to_string()))?;
    let mut graph = state.graph.clone();
    let before = db::get_task(&mut graph, &graph_id, &task_id).await?;
    if db::claim_task(&mut graph, &graph_id, &task_id, &actor.id).await? {
        info!("Task '{}' claimed by '{}'", task_id, actor.name);
        let after = db::get_task(&mut graph, &graph_id, &task_id).await?;
        db::add_events(
            &mut graph,
            &task_id,
            Some(&actor.id),
            &events::diff(&before, &after),
        )
        .await?;
        return Ok(Json(after));
    }
    // Not claimed: say why (404 if there is no such task).
    let task = db::get_task(&mut graph, &graph_id, &task_id).await?;
    let all = db::get_all_tasks(&mut graph, &graph_id).await?;
    let open: Vec<&str> = task
        .depends_on
        .iter()
        .filter_map(|id| all.iter().find(|t| &t.id == id))
        .filter(|t| t.status != Status::Done)
        .map(|t| t.name.as_str())
        .collect();
    let others = task.users.iter().any(|u| u != &actor.id);
    let reason = if task.status != Status::Todo {
        format!("it is {}, not TODO", task.status.as_str())
    } else if others {
        "someone else is assigned to it".to_string()
    } else if !open.is_empty() {
        format!("it is blocked by unfinished tasks: {}", open.join(", "))
    } else {
        "it changed meanwhile; try again".to_string()
    };
    Err(AppError::Conflict(format!(
        "Cannot claim task '{}': {}",
        task.name, reason
    )))
}

/// Rejects making `task_id` depend on any of `depends_on` if that would
/// close a `DEPENDS_ON` cycle.
async fn ensure_no_cycle(
    graph: &mut AsyncGraph,
    task_id: &str,
    depends_on: &[String],
) -> Result<()> {
    if let Some(dep) = db::find_cycle(graph, task_id, depends_on).await? {
        return Err(AppError::Conflict(format!(
            "Task {} already depends on {}, the dependency would create a cycle",
            dep, task_id
        )));
    }
    Ok(())
}

/// Saves where task nodes sit on the canvas. Every task must be in the graph (else 404, nothing
/// saved) and every coordinate finite (else 400).
async fn save_layout(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Json(update): Json<LayoutUpdate>,
) -> Result<Json<MessageResponse>> {
    if update
        .positions
        .values()
        .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Err(AppError::BadRequest(
            "Positions must be finite numbers".to_string(),
        ));
    }
    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    let ids: Vec<String> = update.positions.keys().cloned().collect();
    db::ensure_tasks_in_graph(&mut graph, &graph_id, &ids).await?;
    let positions: Vec<(String, schemas::Position)> = update.positions.into_iter().collect();
    db::save_positions(&mut graph, &graph_id, &positions).await?;
    Ok(Json(MessageResponse {
        message: format!("Saved {} position(s)", positions.len()),
    }))
}

/// Longest comment accepted, in characters.
const MAX_COMMENT_CHARS: usize = 20_000;

async fn list_comments(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
) -> Result<Json<Vec<Comment>>> {
    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    Ok(Json(db::list_comments(&mut graph, &task_id).await?))
}

/// Adds a comment by the caller: 401 without a token, since a comment's author is an actor.
async fn add_comment(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
    Extension(caller): Extension<auth::Caller>,
    Json(comment): Json<NewComment>,
) -> Result<(StatusCode, Json<Comment>)> {
    let author = caller
        .0
        .ok_or_else(|| AppError::Unauthorized("Commenting needs a bearer token".to_string()))?;
    let body = comment.body.trim();
    if body.is_empty() {
        return Err(AppError::BadRequest("Comment cannot be empty".to_string()));
    }
    if body.chars().count() > MAX_COMMENT_CHARS {
        return Err(AppError::BadRequest(format!(
            "Comment is longer than {} characters",
            MAX_COMMENT_CHARS
        )));
    }
    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    let id = Uuid::new_v4().to_string();
    db::add_comment(&mut graph, &task_id, &id, body, &author.id).await?;
    let created = db::list_comments(&mut graph, &task_id)
        .await?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| AppError::Internal(format!("Comment {} vanished", id)))?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn delete_task(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
) -> Result<Json<MessageResponse>> {
    info!("Deleting task '{}' from graph '{}'", task_id, graph_id);
    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    db::delete_task(&mut graph, &task_id).await?;
    db::touch_graph(&mut graph, &graph_id).await?;
    Ok(Json(MessageResponse {
        message: format!("Task {} deleted", task_id),
    }))
}

async fn connect_nodes(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Extension(caller): Extension<auth::Caller>,
    Json(request): Json<ConnectNodesRequest>,
) -> Result<Json<MessageResponse>> {
    info!(
        "Connecting '{}' -[:DEPENDS_ON]-> '{}' in graph '{}'",
        request.node_dependee, request.node_dependency, graph_id
    );
    if request.node_dependee == request.node_dependency {
        return Err(AppError::BadRequest(
            "A task cannot depend on itself".to_string(),
        ));
    }

    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    db::ensure_tasks_in_graph(
        &mut graph,
        &graph_id,
        &[
            request.node_dependee.clone(),
            request.node_dependency.clone(),
        ],
    )
    .await?;
    ensure_no_cycle(
        &mut graph,
        &request.node_dependee,
        std::slice::from_ref(&request.node_dependency),
    )
    .await?;
    let before = db::get_task(&mut graph, &graph_id, &request.node_dependee).await?;
    db::connect(&mut graph, &request.node_dependee, &request.node_dependency).await?;
    db::touch_task(&mut graph, &request.node_dependee).await?;
    if !before.depends_on.contains(&request.node_dependency) {
        let change = Change::blocker(&request.node_dependency, true);
        db::add_events(
            &mut graph,
            &request.node_dependee,
            actor_id(&caller),
            &[change],
        )
        .await?;
    }
    Ok(Json(MessageResponse {
        message: format!(
            "Nodes {} and {} connected",
            request.node_dependency, request.node_dependee
        ),
    }))
}

async fn disconnect_nodes(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Extension(caller): Extension<auth::Caller>,
    Json(request): Json<ConnectNodesRequest>,
) -> Result<Json<MessageResponse>> {
    info!(
        "Disconnecting '{}' -[:DEPENDS_ON]-> '{}' in graph '{}'",
        request.node_dependee, request.node_dependency, graph_id
    );
    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    db::ensure_tasks_in_graph(
        &mut graph,
        &graph_id,
        &[
            request.node_dependee.clone(),
            request.node_dependency.clone(),
        ],
    )
    .await?;
    if !db::disconnect(&mut graph, &request.node_dependee, &request.node_dependency).await? {
        return Err(AppError::NotFound(format!(
            "No dependency {} -> {}",
            request.node_dependee, request.node_dependency
        )));
    }
    db::touch_task(&mut graph, &request.node_dependee).await?;
    let change = Change::blocker(&request.node_dependency, false);
    db::add_events(
        &mut graph,
        &request.node_dependee,
        actor_id(&caller),
        &[change],
    )
    .await?;
    Ok(Json(MessageResponse {
        message: format!(
            "Nodes {} and {} disconnected",
            request.node_dependency, request.node_dependee
        ),
    }))
}
