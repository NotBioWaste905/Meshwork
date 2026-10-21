//! HTTP-level tests against a real FalkorDB.
//!
//! They are ignored by default; run them with a FalkorDB on hand:
//!
//!     docker compose up -d falkordb
//!     cargo test -- --ignored
//!
//! `FALKORDB_URL` defaults to `falkor://localhost:6379`. Each test works in its
//! own throwaway FalkorDB graph, deleted at the end.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use backend_rs::{app, cors_layer, schemas::Task, AppState};
use falkordb::{AsyncGraph, FalkorClientBuilder, FalkorConnectionInfo};
use serde_json::{json, Value};
use tower::ServiceExt;

struct TestApp {
    router: Router,
    graph: AsyncGraph,
}

impl TestApp {
    async fn new() -> Self {
        let uri =
            std::env::var("FALKORDB_URL").unwrap_or_else(|_| "falkor://localhost:6379".to_string());
        let info: FalkorConnectionInfo = uri.as_str().try_into().expect("bad FALKORDB_URL");
        let client = FalkorClientBuilder::new_async()
            .with_connection_info(info)
            .build()
            .await
            .expect("FalkorDB is not reachable");
        let graph = client.select_graph(format!("test_{}", uuid::Uuid::new_v4().simple()));
        let state = AppState {
            graph: graph.clone(),
        };
        TestApp {
            router: app(state, cors_layer("http://localhost:3000")),
            graph,
        }
    }

    async fn call(&self, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let request = Request::builder().method(method).uri(uri);
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        }
        .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    async fn create_graph(&self) -> String {
        let (status, body) = self.call(Method::POST, "/v0/create_graph", None).await;
        assert_eq!(status, StatusCode::OK);
        body["ID"].as_str().unwrap().to_string()
    }

    async fn add_task(&self, graph_id: &str, body: Value) -> Task {
        let (status, body) = self
            .call(
                Method::POST,
                &format!("/v0/{graph_id}/task/add/"),
                Some(body),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        serde_json::from_value(body).unwrap()
    }

    async fn all_tasks(&self, graph_id: &str) -> Vec<Task> {
        let (status, body) = self
            .call(Method::GET, &format!("/v0/{graph_id}/all_tasks"), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        serde_json::from_value(body).unwrap()
    }

    async fn cleanup(mut self) {
        let _ = self.graph.delete().await;
    }
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn create_task_with_minimal_body() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;

    let task = app.add_task(&graph_id, json!({ "name": "New Task" })).await;
    assert!(!task.id.is_empty());
    assert_eq!(task.name, "New Task");
    assert_eq!(task.description, "");
    assert_eq!(serde_json::to_value(task.status).unwrap(), json!("TODO"));
    assert_eq!(task.graph_id, graph_id);
    assert!(task.depends_on.is_empty() && task.users.is_empty() && task.tags.is_empty());

    assert_eq!(app.all_tasks(&graph_id).await, vec![task]);
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn full_task_roundtrip() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;

    let blocker = app.add_task(&graph_id, json!({ "name": "Backend" })).await;
    let task = app
        .add_task(
            &graph_id,
            json!({
                "name": "Integrate",
                "description": "front + back",
                "status": "IN_PROGRESS",
                "tags": ["v0.1", "infra"],
                "users": ["andrew"],
                "depends_on": [blocker.id],
            }),
        )
        .await;
    assert_eq!(task.depends_on, vec![blocker.id.clone()]);
    assert_eq!(task.users, vec!["andrew".to_string()]);
    assert_eq!(task.tags, vec!["v0.1".to_string(), "infra".to_string()]);

    let (status, body) = app
        .call(
            Method::GET,
            &format!("/v0/{graph_id}/task/{}", task.id),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_value::<Task>(body).unwrap(), task);
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn edit_task_partial_and_whole() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let other = app.add_task(&graph_id, json!({ "name": "Other" })).await;
    let task = app
        .add_task(&graph_id, json!({ "name": "Task", "tags": ["a"] }))
        .await;
    let uri = format!("/v0/{graph_id}/task/{}", task.id);

    // Partial: only the status changes.
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "status": "REVIEW" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "REVIEW");
    assert_eq!(body["name"], "Task");
    assert_eq!(body["tags"], json!(["a"]));

    // Whole task as the frontend sends it, with extra fields.
    let mut whole = body.clone();
    whole["name"] = json!("Renamed");
    whole["users"] = json!(["bob"]);
    whole["depends_on"] = json!([other.id]);
    whole["graphId"] = json!(graph_id);
    let (status, body) = app.call(Method::PUT, &uri, Some(whole)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Renamed");
    assert_eq!(body["users"], json!(["bob"]));
    assert_eq!(body["depends_on"], json!([other.id]));

    let (status, _) = app
        .call(Method::PUT, &uri, Some(json!({ "depends_on": [task.id] })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = app
        .call(Method::PUT, &uri, Some(json!({ "name": "  " })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn connect_disconnect_and_delete() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let a = app.add_task(&graph_id, json!({ "name": "A" })).await;
    let b = app.add_task(&graph_id, json!({ "name": "B" })).await;
    // Edge as drawn on the canvas: source (blocker) -> target (dependee).
    let edge = json!({ "node_dependency": a.id, "node_dependee": b.id });

    let connect = format!("/v0/{graph_id}/connect_nodes/");
    let disconnect = format!("/v0/{graph_id}/disconnect_nodes/");
    let (status, _) = app.call(Method::POST, &connect, Some(edge.clone())).await;
    assert_eq!(status, StatusCode::OK);
    // Connecting twice does not duplicate the edge.
    app.call(Method::POST, &connect, Some(edge.clone())).await;
    let tasks = app.all_tasks(&graph_id).await;
    let b_now = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(b_now.depends_on, vec![a.id.clone()]);

    let (status, _) = app
        .call(Method::POST, &disconnect, Some(edge.clone()))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .call(Method::POST, &disconnect, Some(edge.clone()))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let self_edge = json!({ "node_dependency": a.id, "node_dependee": a.id });
    let (status, _) = app.call(Method::POST, &connect, Some(self_edge)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    app.call(Method::POST, &connect, Some(edge)).await;
    let (status, _) = app
        .call(
            Method::DELETE,
            &format!("/v0/{graph_id}/task/{}", a.id),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let tasks = app.all_tasks(&graph_id).await;
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].depends_on.is_empty());
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn not_found_and_bad_requests() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let missing = uuid::Uuid::new_v4();

    let cases = [
        (Method::GET, format!("/v0/{missing}/all_tasks"), None),
        (
            Method::POST,
            format!("/v0/{missing}/task/add/"),
            Some(json!({ "name": "x" })),
        ),
        (Method::GET, format!("/v0/{graph_id}/task/{missing}"), None),
        (
            Method::PUT,
            format!("/v0/{graph_id}/task/{missing}"),
            Some(json!({ "name": "x" })),
        ),
        (
            Method::DELETE,
            format!("/v0/{graph_id}/task/{missing}"),
            None,
        ),
        (
            Method::POST,
            format!("/v0/{graph_id}/task/add/"),
            Some(json!({ "name": "x", "depends_on": [missing.to_string()] })),
        ),
    ];
    for (method, uri, body) in cases {
        let (status, _) = app.call(method.clone(), &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }

    let (status, _) = app
        .call(
            Method::POST,
            &format!("/v0/{graph_id}/task/add/"),
            Some(json!({ "name": "" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A task of another graph is not visible through this one.
    let other_graph = app.create_graph().await;
    let foreign = app
        .add_task(&other_graph, json!({ "name": "foreign" }))
        .await;
    let (status, _) = app
        .call(
            Method::GET,
            &format!("/v0/{graph_id}/task/{}", foreign.id),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn cors_allows_frontend_origin() {
    let app = TestApp::new().await;
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v0/create_graph")
        .header(header::ORIGIN, "http://localhost:3000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap();
    let response = app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "http://localhost:3000"
    );
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn dependency_cycles_are_rejected() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    // c depends on b, b depends on a.
    let a = app.add_task(&graph_id, json!({ "name": "A" })).await;
    let b = app
        .add_task(&graph_id, json!({ "name": "B", "depends_on": [a.id] }))
        .await;
    let c = app
        .add_task(&graph_id, json!({ "name": "C", "depends_on": [b.id] }))
        .await;
    let connect = format!("/v0/{graph_id}/connect_nodes/");

    // a -> c would close a -> c -> b -> a.
    let (status, body) = app
        .call(
            Method::POST,
            &connect,
            Some(json!({ "node_dependency": c.id, "node_dependee": a.id })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // Same through editing, also for a direct two-node cycle.
    let a_uri = format!("/v0/{graph_id}/task/{}", a.id);
    for dep in [&b.id, &c.id] {
        let (status, _) = app
            .call(Method::PUT, &a_uri, Some(json!({ "depends_on": [dep] })))
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    let tasks = app.all_tasks(&graph_id).await;
    assert!(tasks
        .iter()
        .find(|t| t.id == a.id)
        .unwrap()
        .depends_on
        .is_empty());

    // A diamond is fine: c may depend on a directly as well.
    let (status, _) = app
        .call(
            Method::POST,
            &connect,
            Some(json!({ "node_dependency": a.id, "node_dependee": c.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    // Re-saving b with its current dependencies is not a cycle either.
    let (status, _) = app
        .call(
            Method::PUT,
            &format!("/v0/{graph_id}/task/{}", b.id),
            Some(json!({ "depends_on": [a.id] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    app.cleanup().await;
}
