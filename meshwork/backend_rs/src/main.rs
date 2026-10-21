use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use falkordb::{AsyncGraph, FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use uuid::Uuid;
mod schemas;

// Custom error types
#[derive(Debug)]
enum AppError {
    Database(String),
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(e) => {
                error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                )
            }
            AppError::NotFound(msg) => {
                warn!("Resource not found: {}", msg);
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::BadRequest(msg) => {
                warn!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg)
            }
            AppError::Internal(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        (status, Json(MessageResponse { message })).into_response()
    }
}

type Result<T> = std::result::Result<T, AppError>;

// Application state
#[derive(Clone)]
struct AppState {
    graph: AsyncGraph,
}

fn db_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Database(e.to_string())
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("Starting Meshwork Backend Server");

    let uri = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "falkor://falkordb:6379".to_string());
    let graph_name = std::env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "meshwork".to_string());

    info!("Attempting to connect to FalkorDB at {}", uri);
    let connection_info: FalkorConnectionInfo = match uri.as_str().try_into() {
        Ok(info) => info,
        Err(e) => {
            error!("Invalid FALKORDB_URL '{}': {}", uri, e);
            std::process::exit(1);
        }
    };
    let client: FalkorAsyncClient = match FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
    {
        Ok(client) => {
            info!("Successfully connected to FalkorDB");
            client
        }
        Err(e) => {
            error!("Failed to connect to FalkorDB: {}", e);
            error!("Make sure FalkorDB service is running and accessible");
            std::process::exit(1);
        }
    };

    let state = AppState {
        graph: client.select_graph(graph_name),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v0/create_graph", post(create_graph))
        .route("/v0/:graph_id/task/add/", post(add_task))
        .route("/v0/:graph_id/all_tasks", get(get_all_tasks))
        .route("/v0/:graph_id/task/:task_id", get(get_task))
        .route("/v0/:graph_id/task/:task_id", delete(delete_task))
        .route("/v0/:graph_id/task/:task_id", put(edit_task))
        .route("/v0/:graph_id/connect_nodes/", post(connect_nodes))
        .route("/v0/:graph_id/disconnect_nodes/", post(disconnect_nodes))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    // Specify the address to listen on
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("Server running at http://{}", addr);
    // Start the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
// Root handler
async fn root() -> &'static str {
    "Welcome to the Rust Web Server!"
}

// Health check
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "meshwork-backend".to_string(),
    })
}

#[derive(Serialize)]
struct CreateGraphResponse {
    #[serde(rename = "ID")]
    id: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Deserialize)]
struct ConnectNodesRequest {
    node_dependency: String,
    node_dependee: String,
}

async fn create_graph(State(state): State<AppState>) -> Result<Json<CreateGraphResponse>> {
    let id = Uuid::new_v4().to_string();
    info!("Creating new graph with ID: {}", id);

    let mut graph = state.graph.clone();
    graph
        .query("CREATE (:Graph {id: $id})")
        .with_param("id", id.clone())
        .execute()
        .await
        .map_err(db_err)?;

    info!("Graph created successfully with ID: {}", id);
    Ok(Json(CreateGraphResponse { id }))
}

// NOTE: FalkorDB has no multi-query transactions, so every statement below is
// atomic on its own but the handler as a whole is not.
async fn add_task(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(task): Json<schemas::Task>,
) -> Result<Json<MessageResponse>> {
    info!("Adding task '{}' to graph '{}'", task.id, graph_id);

    // Input validation
    if task.id.trim().is_empty() {
        return Err(AppError::BadRequest("Task ID cannot be empty".to_string()));
    }
    if task.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Task name cannot be empty".to_string(),
        ));
    }

    let mut graph = state.graph.clone();

    // Create the task and link it to the graph in one statement; if the graph
    // doesn't exist the MATCH yields no rows and nothing is created.
    let q = "MATCH (g:Graph {id: $graph_id}) \
             CREATE (t:Task {id: $id, name: $name, description: $description, status: $status})-[:PART_OF]->(g) \
             RETURN t.id";
    let mut result = graph
        .query(q)
        .with_params([
            ("graph_id", graph_id.as_str()),
            ("id", task.id.as_str()),
            ("name", task.name.as_str()),
            ("description", task.description.as_str()),
            ("status", &format!("{:?}", task.status)),
        ])
        .execute()
        .await
        .map_err(db_err)?;
    if result.data.next().await.is_none() {
        return Err(AppError::NotFound(format!("Graph {} not found", graph_id)));
    }

    // Create dependencies
    for dependency in &task.depends_on {
        info!("Adding dependency: {} -> {}", task.id, dependency);
        let q = "MATCH (t:Task {id: $id}), (dep_task:Task {id: $dependency_id}) \
                 CREATE (t)-[:DEPENDS_ON]->(dep_task)";
        graph
            .query(q)
            .with_params([("id", task.id.as_str()), ("dependency_id", dependency.as_str())])
            .execute()
            .await
            .map_err(db_err)?;
    }

    // Assign users
    for user in &task.users {
        info!("Assigning user '{}' to task '{}'", user, task.id);
        let q = "MATCH (t:Task {id: $id}) \
                 MERGE (u:User {id: $user_id}) \
                 CREATE (t)-[:ASSIGNED_TO]->(u)";
        graph
            .query(q)
            .with_params([("id", task.id.as_str()), ("user_id", user.as_str())])
            .execute()
            .await
            .map_err(db_err)?;
    }

    info!(
        "Task '{}' added successfully to graph '{}'",
        task.id, graph_id
    );
    Ok(Json(MessageResponse {
        message: format!("Task {} added successfully", task.id),
    }))
}

async fn get_all_tasks(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
) -> Result<Json<Vec<schemas::Task>>> {
    info!("Fetching all tasks for graph '{}'", graph_id);

    if graph_id.trim().is_empty() {
        return Err(AppError::BadRequest("Graph ID cannot be empty".to_string()));
    }

    let q = "MATCH (t:Task)-[:PART_OF]->(g:Graph {id: $id}) \
             OPTIONAL MATCH (t)-[:DEPENDS_ON]->(dep:Task) \
             OPTIONAL MATCH (t)-[:ASSIGNED_TO]->(u:User) \
             RETURN t.id AS id, t.name AS name, t.description AS description, t.status AS status, \
                    collect(DISTINCT dep.id) AS dependencies, \
                    collect(DISTINCT u.id) AS users";

    let mut graph = state.graph.clone();
    let mut result = graph
        .query(q)
        .with_param("id", graph_id.clone())
        .execute()
        .await
        .map_err(db_err)?;

    let mut tasks = Vec::new();

    while let Some(row) = result.data.next().await {
        let row = row.map_err(db_err)?;
        let status: String = row.try_get("status").map_err(db_err)?;

        tasks.push(schemas::Task {
            id: row.try_get("id").map_err(db_err)?,
            name: row.try_get("name").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            status: serde_json::from_str(&format!("\"{}\"", status))
                .map_err(|e| AppError::Internal(format!("Bad status '{}': {}", status, e)))?,
            depends_on: row.try_get("dependencies").map_err(db_err)?,
            users: row.try_get("users").map_err(db_err)?,
            tags: vec![], // You'll need to handle tags similarly if you use them
            graph_id: graph_id.clone(),
        });
    }

    Ok(Json(tasks))
}

async fn get_task(
    State(_state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<schemas::Task>> {
    info!("Fetching task '{}' from graph '{}'", task_id, graph_id);

    // TODO: Implement actual database query
    warn!("get_task not yet implemented - returning dummy task");
    Ok(Json(schemas::Task {
        id: task_id,
        name: "Dummy Task".to_string(),
        description: "Placeholder".to_string(),
        depends_on: vec![],
        users: vec![],
        tags: vec![],
        status: schemas::Status::TODO,
        graph_id,
    }))
}

async fn delete_task(
    State(_state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
) -> Result<Json<MessageResponse>> {
    info!("Deleting task '{}' from graph '{}'", task_id, graph_id);

    // TODO: Implement actual database deletion
    warn!("delete_task not yet implemented");
    Ok(Json(MessageResponse {
        message: format!("Task {} deleted", task_id),
    }))
}

async fn edit_task(
    State(_state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
    Json(_new_task): Json<schemas::Task>,
) -> Result<Json<MessageResponse>> {
    info!("Editing task '{}' in graph '{}'", task_id, graph_id);

    // TODO: Implement actual database update
    warn!("edit_task not yet implemented");
    Ok(Json(MessageResponse {
        message: format!("Task {} edited", task_id),
    }))
}

async fn connect_nodes(
    State(_state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(request): Json<ConnectNodesRequest>,
) -> Result<Json<MessageResponse>> {
    info!(
        "Connecting nodes '{}' -> '{}' in graph '{}'",
        request.node_dependency, request.node_dependee, graph_id
    );

    // TODO: Implement actual database connection
    warn!("connect_nodes not yet implemented");
    Ok(Json(MessageResponse {
        message: format!(
            "Nodes {} and {} connected",
            request.node_dependency, request.node_dependee
        ),
    }))
}

async fn disconnect_nodes(
    State(_state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(request): Json<ConnectNodesRequest>,
) -> Result<Json<MessageResponse>> {
    info!(
        "Disconnecting nodes '{}' -> '{}' in graph '{}'",
        request.node_dependency, request.node_dependee, graph_id
    );

    // TODO: Implement actual database disconnection
    warn!("disconnect_nodes not yet implemented");
    Ok(Json(MessageResponse {
        message: format!(
            "Nodes {} and {} disconnected",
            request.node_dependency, request.node_dependee
        ),
    }))
}
