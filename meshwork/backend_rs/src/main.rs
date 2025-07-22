use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Json, Router,
};
use neo4rs::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;
mod schemas;

// Application state
#[derive(Clone)]
struct AppState {
    graph: Arc<Graph>,
}

#[tokio::main]
async fn main() {
    // Build the router
    let uri = "neo4j:7687";
    let user = "neo4j";
    let pass = "your_password";

    println!("Attempting to connect to Neo4j at {}", uri);
    let graph = match Graph::new(uri, user, pass).await {
        Ok(graph) => {
            println!("Successfully connected to Neo4j");
            graph
        }
        Err(e) => {
            eprintln!("Failed to connect to Neo4j: {}", e);
            eprintln!("Make sure Neo4j service is running and accessible");
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
        .with_state(state);
    // Specify the address to listen on
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    println!("Server running at http://{}", addr);
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

async fn create_graph(State(state): State<AppState>) -> Json<CreateGraphResponse> {
    let id = Uuid::new_v4().to_string();
    let mut txn = state.graph.start_txn().await.unwrap();
    let q = "CREATE (n:Graph {id: $id}) RETURN n";
    txn.run(query(q).param("id", id.clone())).await.unwrap();
    txn.commit().await.unwrap();
    Json(CreateGraphResponse { id })
}

async fn add_task(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(task): Json<schemas::Task>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: format!("Task {} added", task.id),
    })
}

async fn get_all_tasks(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
) -> Json<Vec<schemas::Task>> {
    // Return empty vector for now - implementation needed
    Json(vec![])
}

async fn get_task(
    State(state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
) -> Json<schemas::Task> {
    // Return dummy task for now - implementation needed
    Json(schemas::Task {
        id: task_id,
        name: "Dummy Task".to_string(),
        description: "Placeholder".to_string(),
        depends_on: vec![],
        users: vec![],
        tags: vec![],
        status: schemas::Status::TODO,
        graph_id,
    })
}

async fn delete_task(
    State(state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: format!("Task {} deleted", task_id),
    })
}

async fn edit_task(
    State(state): State<AppState>,
    axum::extract::Path((graph_id, task_id)): axum::extract::Path<(String, String)>,
    Json(new_task): Json<schemas::Task>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: format!("Task {} edited", task_id),
    })
}

async fn connect_nodes(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(request): Json<ConnectNodesRequest>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: format!(
            "Nodes {} and {} connected",
            request.node_dependency, request.node_dependee
        ),
    })
}

async fn disconnect_nodes(
    State(state): State<AppState>,
    axum::extract::Path(graph_id): axum::extract::Path<String>,
    Json(request): Json<ConnectNodesRequest>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: format!(
            "Nodes {} and {} disconnected",
            request.node_dependency, request.node_dependee
        ),
    })
}
