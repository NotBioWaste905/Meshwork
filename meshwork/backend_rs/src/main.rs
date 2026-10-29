use backend_rs::{app, connection, cors_layer, AppState};
use std::net::SocketAddr;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    info!("Starting Meshwork Backend Server");

    let uri =
        std::env::var("FALKORDB_URL").unwrap_or_else(|_| "falkor://falkordb:6379".to_string());
    let graph_name = std::env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "meshwork".to_string());
    let cors_origins = std::env::var("CORS_ALLOW_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string());

    info!("Attempting to connect to FalkorDB at {}", uri);
    let client = match connection::connect(&uri).await {
        Ok(client) => {
            info!("Successfully connected to FalkorDB");
            client
        }
        Err(e) => {
            error!("Failed to connect to FalkorDB at '{}': {:?}", uri, e);
            error!("Make sure FalkorDB service is running and accessible");
            std::process::exit(1);
        }
    };

    let mut graph = client.select_graph(graph_name);
    if let Err(e) = connection::ensure_indexes(&mut graph).await {
        error!("Failed to create FalkorDB indexes: {:?}", e);
        std::process::exit(1);
    }
    if let Err(e) = connection::migrate(&mut graph).await {
        error!("Failed to migrate FalkorDB data: {:?}", e);
        std::process::exit(1);
    }

    let allow_anonymous = match std::env::var("MESHWORK_ALLOW_ANONYMOUS") {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "" | "false" | "0" | "no" => false,
            other => {
                error!(
                    "MESHWORK_ALLOW_ANONYMOUS must be true or false, got '{}'",
                    other
                );
                std::process::exit(1);
            }
        },
        Err(_) => false,
    };
    if allow_anonymous {
        warn!(
            "MESHWORK_ALLOW_ANONYMOUS=true: /v0/ requests without a token are served as nobody. \
             For local development only."
        );
    } else {
        info!("Every /v0/ request needs an actor token (MESHWORK_ALLOW_ANONYMOUS is off)");
    }

    let state = AppState {
        graph,
        allow_anonymous,
    };
    info!("CORS allowed origins: {}", cors_origins);
    let app = app(state, cors_layer(&cors_origins));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Server running at http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
