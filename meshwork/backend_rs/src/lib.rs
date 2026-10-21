// The falkordb query futures are deeply nested; rustc >= 1.9x needs more than the default 128.
#![recursion_limit = "256"]

use axum::{
    extract::{Path, State},
    http::{HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use falkordb::AsyncGraph;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use uuid::Uuid;

mod db;
pub mod error;
pub mod schemas;

use error::{AppError, Result};
use schemas::{
    ConnectNodesRequest, CreateGraphResponse, MessageResponse, NewTask, Task, TaskUpdate,
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
        .route("/v0/:graph_id/task/add/", post(add_task))
        .route("/v0/:graph_id/all_tasks", get(get_all_tasks))
        .route(
            "/v0/:graph_id/task/:task_id",
            get(get_task).put(edit_task).delete(delete_task),
        )
        .route("/v0/:graph_id/connect_nodes/", post(connect_nodes))
        .route("/v0/:graph_id/disconnect_nodes/", post(disconnect_nodes))
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

async fn create_graph(State(state): State<AppState>) -> Result<Json<CreateGraphResponse>> {
    let id = Uuid::new_v4().to_string();
    info!("Creating new graph with ID: {}", id);
    let mut graph = state.graph.clone();
    db::create_graph(&mut graph, &id).await?;
    Ok(Json(CreateGraphResponse { id }))
}

async fn add_task(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
    Json(task): Json<NewTask>,
) -> Result<(StatusCode, Json<Task>)> {
    if task.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Task name cannot be empty".to_string(),
        ));
    }

    let mut graph = state.graph.clone();
    db::ensure_graph(&mut graph, &graph_id).await?;
    db::ensure_tasks_in_graph(&mut graph, &graph_id, &task.depends_on).await?;

    let id = Uuid::new_v4().to_string();
    info!("Adding task '{}' to graph '{}'", id, graph_id);
    db::create_task(
        &mut graph,
        &graph_id,
        &id,
        &task.name,
        &task.description,
        task.status,
        &task.tags,
    )
    .await?;
    db::set_dependencies(&mut graph, &id, &task.depends_on).await?;
    db::set_users(&mut graph, &id, &task.users).await?;

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
    Json(update): Json<TaskUpdate>,
) -> Result<Json<Task>> {
    info!("Editing task '{}' in graph '{}'", task_id, graph_id);
    if matches!(&update.name, Some(name) if name.trim().is_empty()) {
        return Err(AppError::BadRequest(
            "Task name cannot be empty".to_string(),
        ));
    }

    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    if let Some(deps) = &update.depends_on {
        if deps.contains(&task_id) {
            return Err(AppError::BadRequest(
                "A task cannot depend on itself".to_string(),
            ));
        }
        db::ensure_tasks_in_graph(&mut graph, &graph_id, deps).await?;
    }

    db::update_task_props(
        &mut graph,
        &task_id,
        update.name.as_deref(),
        update.description.as_deref(),
        update.status,
        update.tags.as_deref(),
    )
    .await?;
    if let Some(deps) = &update.depends_on {
        db::set_dependencies(&mut graph, &task_id, deps).await?;
    }
    if let Some(users) = &update.users {
        db::set_users(&mut graph, &task_id, users).await?;
    }

    Ok(Json(db::get_task(&mut graph, &graph_id, &task_id).await?))
}

async fn delete_task(
    State(state): State<AppState>,
    Path((graph_id, task_id)): Path<(String, String)>,
) -> Result<Json<MessageResponse>> {
    info!("Deleting task '{}' from graph '{}'", task_id, graph_id);
    let mut graph = state.graph.clone();
    db::get_task(&mut graph, &graph_id, &task_id).await?;
    db::delete_task(&mut graph, &task_id).await?;
    Ok(Json(MessageResponse {
        message: format!("Task {} deleted", task_id),
    }))
}

async fn connect_nodes(
    State(state): State<AppState>,
    Path(graph_id): Path<String>,
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
    db::connect(&mut graph, &request.node_dependee, &request.node_dependency).await?;
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
    Ok(Json(MessageResponse {
        message: format!(
            "Nodes {} and {} disconnected",
            request.node_dependency, request.node_dependee
        ),
    }))
}
