use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use neo4rs::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use uuid::Uuid;
mod schemas;

// Custom error types
#[derive(Debug)]
enum AppError {
    Database(neo4rs::Error),
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
    graph: Arc<Graph>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("Starting Meshwork Backend Server");

    // Build the router
    let uri = "neo4j:7687";
    let user = "neo4j";
    let pass = "your_password";

    info!("Attempting to connect to Neo4j at {}", uri);
    let graph = match Graph::new(uri, user, pass).await {
        Ok(graph) => {
            info!("Successfully connected to Neo4j");
            graph
        }
        Err(e) => {
            error!("Failed to connect to Neo4j: {}", e);
            error!("Make sure Neo4j service is running and accessible");
            std::process::exit(1);
        }
    };

    let state = AppState {
        graph: Arc::new(graph),
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

    let mut txn = state.graph.start_txn().await.map_err(AppError::Database)?;

    let q = "CREATE (n:Graph {id: $id}) RETURN n";
    txn.run(query(q).param("id", id.clone()))
        .await
        .map_err(AppError::Database)?;

    txn.commit().await.map_err(AppError::Database)?;

    info!("Graph created successfully with ID: {}", id);
    Ok(Json(CreateGraphResponse { id }))
}

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

    let mut txn = state.graph.start_txn().await.map_err(AppError::Database)?;

    // Create the task
    let q = "CREATE (t:Task {id: $id, name: $name, description: $description, status: $status}) RETURN t";
    txn.run(
        query(q)
            .param("id", task.id.clone())
            .param("name", task.name.clone())
            .param("description", task.description.clone())
            .param("status", format!("{:?}", task.status)),
    )
    .await
    .map_err(AppError::Database)?;

    // Link task to graph
    let q = "MATCH (t:Task {id: $id}), (g:Graph {id: $graph_id}) CREATE (t)-[rel:PART_OF]->(g) RETURN t";
    txn.run(
        query(q)
            .param("id", task.id.clone())
            .param("graph_id", graph_id.clone()),
    )
    .await
    .map_err(AppError::Database)?;

    // Create dependencies
    for dependency in &task.depends_on {
        info!("Adding dependency: {} -> {}", task.id, dependency);
        let q = "MATCH (t:Task {id: $id}), (dep_task:Task {id: $dependency_id}) CREATE (t)-[rel:DEPENDS_ON]->(dep_task) RETURN t";
        txn.run(
            query(q)
                .param("id", task.id.clone())
                .param("dependency_id", dependency.clone()),
        )
        .await
        .map_err(AppError::Database)?;
    }

    // Assign users
    for user in &task.users {
        info!("Assigning user '{}' to task '{}'", user, task.id);
        let q = "MERGE (user:User {id: $user_id}) WITH user MATCH (t:Task {id: $id}) CREATE (t)-[rel:ASSIGNED_TO]->(user) RETURN t";
        txn.run(
            query(q)
                .param("id", task.id.clone())
                .param("user_id", user.clone()),
        )
        .await
        .map_err(AppError::Database)?;
    }

    txn.commit().await.map_err(AppError::Database)?;

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

    let q = r#"
        MATCH (t:Task)-[:PART_OF]->(g:Graph {id: $id})
        OPTIONAL MATCH (t)-[:DEPENDS_ON]->(dep:Task)
        OPTIONAL MATCH (t)-[:ASSIGNED_TO]->(u:User)
        RETURN t,
               collect(DISTINCT dep.id) as dependencies,
               collect(DISTINCT u.id) as users
    "#;

    let mut result = state
        .graph
        .execute(query(q).param("id", graph_id.clone()))
        .await
        .map_err(AppError::Database)?;

    let mut tasks = Vec::new();

    while let Ok(Some(record)) = result.next().await {
        let node: neo4rs::Node = record.get("t").unwrap();
        let dependencies: Vec<String> = record.get("dependencies").unwrap_or_default();
        let users: Vec<String> = record.get("users").unwrap_or_default();

        let task = schemas::Task {
            id: node.get("id").unwrap(),
            name: node.get("name").unwrap(),
            description: node.get("description").unwrap(),
            status: serde_json::from_str(&format!("\"{}\"", node.get::<String>("status").unwrap()))
                .unwrap(),
            depends_on: dependencies,
            users,
            tags: vec![], // You'll need to handle tags similarly if you use them
            graph_id: graph_id.clone(),
        };

        tasks.push(task);
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
