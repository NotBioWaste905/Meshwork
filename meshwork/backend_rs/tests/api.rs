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
use backend_rs::{
    app, auth, connection, cors_layer,
    schemas::{Actor, ActorKind, Task},
    AppState,
};
use falkordb::AsyncGraph;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::{net::TcpListener, task::JoinHandle};
use tower::ServiceExt;

fn falkordb_url() -> String {
    std::env::var("FALKORDB_URL").unwrap_or_else(|_| "falkor://localhost:6379".to_string())
}

struct TestApp {
    router: Router,
    graph: AsyncGraph,
}

impl TestApp {
    async fn new() -> Self {
        Self::with_url(&falkordb_url()).await
    }

    async fn with_url(uri: &str) -> Self {
        let client = connection::connect(uri)
            .await
            .expect("FalkorDB is not reachable");
        let mut graph = client.select_graph(format!("test_{}", uuid::Uuid::new_v4().simple()));
        connection::ensure_indexes(&mut graph)
            .await
            .expect("cannot create indexes");
        let state = AppState {
            graph: graph.clone(),
        };
        TestApp {
            router: app(state, cors_layer("http://localhost:3000")),
            graph,
        }
    }

    async fn call(&self, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        self.call_as(None, method, uri, body).await
    }

    /// Like [`call`](Self::call), with `Authorization: Bearer <token>` when a token is given.
    async fn call_as(
        &self,
        token: Option<&str>,
        method: Method,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
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

    async fn add_actor(&self, body: Value) -> Actor {
        let (status, body) = self.call(Method::POST, "/v0/actors", Some(body)).await;
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
    let andrew = app.add_actor(json!({ "name": "andrew" })).await;
    let task = app
        .add_task(
            &graph_id,
            json!({
                "name": "Integrate",
                "description": "front + back",
                "status": "IN_PROGRESS",
                "tags": ["v0.1", "infra"],
                "users": [andrew.id],
                "depends_on": [blocker.id],
            }),
        )
        .await;
    assert_eq!(task.depends_on, vec![blocker.id.clone()]);
    assert_eq!(task.users, vec![andrew.id.clone()]);
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
    let bob = app.add_actor(json!({ "name": "bob" })).await;

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
    // Assigned by name, returned as the actor's ID.
    assert_eq!(body["users"], json!([bob.id]));
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
async fn priority_and_pr_url() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;

    let plain = app.add_task(&graph_id, json!({ "name": "Plain" })).await;
    assert_eq!(
        serde_json::to_value(plain.priority).unwrap(),
        json!("NORMAL")
    );
    assert_eq!(plain.pr_url, "");

    let urgent = app
        .add_task(
            &graph_id,
            json!({
                "name": "Urgent",
                "priority": "HIGH",
                "pr_url": "https://github.com/o/r/pull/1",
            }),
        )
        .await;
    assert_eq!(
        serde_json::to_value(urgent.priority).unwrap(),
        json!("HIGH")
    );
    assert_eq!(urgent.pr_url, "https://github.com/o/r/pull/1");

    // Partial edits; an empty link removes it.
    let uri = format!("/v0/{graph_id}/task/{}", urgent.id);
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "priority": "LOW" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["priority"], "LOW");
    assert_eq!(body["pr_url"], "https://github.com/o/r/pull/1");
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "pr_url": "" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["pr_url"], "");

    // Only http(s) links, so the UI never renders e.g. a javascript: link.
    for bad in ["javascript:alert(1)", "github.com/o/r/pull/1"] {
        let (status, _) = app
            .call(Method::PUT, &uri, Some(json!({ "pr_url": bad })))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        let (status, _) = app
            .call(
                Method::POST,
                &format!("/v0/{graph_id}/task/add/"),
                Some(json!({ "name": "X", "pr_url": bad })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    // Tasks stored before these fields existed read as NORMAL without a link.
    app.graph
        .query("MATCH (t:Task {id: $id}) REMOVE t.priority, t.pr_url")
        .with_param("id", plain.id.as_str())
        .execute()
        .await
        .unwrap();
    let (status, body) = app
        .call(
            Method::GET,
            &format!("/v0/{graph_id}/task/{}", plain.id),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        (&body["priority"], &body["pr_url"]),
        (&json!("NORMAL"), &json!(""))
    );

    // The frontier lists higher priorities first.
    let high = app
        .add_task(&graph_id, json!({ "name": "High", "priority": "HIGH" }))
        .await;
    let (status, body) = app
        .call(Method::GET, &format!("/v0/{graph_id}/ready_tasks"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<String> = serde_json::from_value::<Vec<Task>>(body)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names.first(), Some(&high.name));
    assert_eq!(names.last(), Some(&"Urgent".to_string()));

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

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn ready_tasks_follow_dependencies() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let a = app.add_task(&graph_id, json!({ "name": "A" })).await;
    let b = app
        .add_task(&graph_id, json!({ "name": "B", "depends_on": [a.id] }))
        .await;
    let alice = app.add_actor(json!({ "name": "alice" })).await;
    let c = app
        .add_task(&graph_id, json!({ "name": "C", "users": [alice.id] }))
        .await;
    let ready = |unassigned_only: bool| {
        let uri = format!("/v0/{graph_id}/ready_tasks?unassigned_only={unassigned_only}");
        let app = &app;
        async move {
            let (status, body) = app.call(Method::GET, &uri, None).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            let mut ids: Vec<String> = serde_json::from_value::<Vec<Task>>(body)
                .unwrap()
                .into_iter()
                .map(|t| t.id)
                .collect();
            ids.sort();
            ids
        }
    };
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };

    assert_eq!(ready(false).await, sorted(vec![a.id.clone(), c.id.clone()]));
    assert_eq!(ready(true).await, vec![a.id.clone()]);

    // Finishing A unblocks B; A itself is no longer TODO.
    app.call(
        Method::PUT,
        &format!("/v0/{graph_id}/task/{}", a.id),
        Some(json!({ "status": "DONE" })),
    )
    .await;
    assert_eq!(ready(true).await, vec![b.id.clone()]);

    // The flag is optional.
    let (status, _) = app
        .call(Method::GET, &format!("/v0/{graph_id}/ready_tasks"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .call(
            Method::GET,
            &format!("/v0/{}/ready_tasks", uuid::Uuid::new_v4()),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    app.cleanup().await;
}

/// A TCP proxy in front of FalkorDB whose connections can be cut on demand,
/// which looks to the client exactly like a FalkorDB restart.
struct CuttableProxy {
    port: u16,
    conns: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl CuttableProxy {
    async fn start(upstream: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let conns: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::default();
        let registry = conns.clone();
        tokio::spawn(async move {
            while let Ok((mut client, _)) = listener.accept().await {
                let upstream = upstream.clone();
                let handle = tokio::spawn(async move {
                    if let Ok(mut server) = tokio::net::TcpStream::connect(upstream).await {
                        let _ = tokio::io::copy_bidirectional(&mut client, &mut server).await;
                    }
                });
                registry.lock().unwrap().push(handle);
            }
        });
        CuttableProxy { port, conns }
    }

    /// Drops every proxied connection; new ones are still accepted.
    fn cut(&self) {
        for handle in self.conns.lock().unwrap().drain(..) {
            handle.abort();
        }
    }
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn recovers_after_connection_loss() {
    let upstream = falkordb_url()
        .split("://")
        .last()
        .unwrap()
        .trim_end_matches('/')
        .to_string();
    let proxy = CuttableProxy::start(upstream).await;
    let app = TestApp::with_url(&format!("falkor://127.0.0.1:{}", proxy.port)).await;
    let graph_id = app.create_graph().await;
    let before = app.add_task(&graph_id, json!({ "name": "Before" })).await;

    // A write right after the drop must not fail and must not be applied twice.
    proxy.cut();
    let after = app.add_task(&graph_id, json!({ "name": "After" })).await;

    proxy.cut();
    let mut ids: Vec<String> = app
        .all_tasks(&graph_id)
        .await
        .into_iter()
        .map(|t| t.id)
        .collect();
    ids.sort();
    let mut expected = vec![before.id, after.id];
    expected.sort();
    assert_eq!(ids, expected);
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn ensure_indexes_is_idempotent() {
    let mut app = TestApp::new().await;
    // `TestApp::new` already created them once.
    connection::ensure_indexes(&mut app.graph).await.unwrap();
    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn graph_metadata_and_listing() {
    let app = TestApp::new().await;

    // No body, an empty JSON object and a named graph are all accepted.
    let unnamed = app.create_graph().await;
    // `created_at` has millisecond resolution; keep the creation order unambiguous.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let (status, body) = app
        .call(Method::POST, "/v0/create_graph", Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], json!(""));
    let empty_obj = body["ID"].as_str().unwrap().to_string();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let (status, body) = app
        .call(
            Method::POST,
            "/v0/create_graph",
            Some(json!({ "name": "  Ship billing  " })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], json!("Ship billing"));
    let named = body["ID"].as_str().unwrap().to_string();

    let (status, body) = app
        .call(Method::POST, "/v0/create_graph", Some(json!("nope")))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let a = app.add_task(&named, json!({ "name": "A" })).await;
    let b = app
        .add_task(&named, json!({ "name": "B", "status": "DONE" }))
        .await;
    app.add_task(
        &named,
        json!({ "name": "C", "status": "BLOCKED", "depends_on": [a.id, b.id] }),
    )
    .await;

    let (status, body) = app.call(Method::GET, &format!("/v0/{named}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let counts = |b: &Value| {
        json!({
            "id": b["id"], "name": b["name"], "task_count": b["task_count"],
            "done_count": b["done_count"], "blocked_count": b["blocked_count"]
        })
    };
    assert_eq!(
        counts(&body),
        json!({
            "id": named, "name": "Ship billing",
            "task_count": 3, "done_count": 1, "blocked_count": 1
        })
    );
    assert!(body["updated_at"].as_i64() >= body["created_at"].as_i64());

    let (status, body) = app.call(Method::GET, "/v0/graphs", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let graphs = body.as_array().unwrap();
    assert_eq!(graphs.len(), 3);
    // Newest first.
    assert_eq!(graphs[0]["id"], json!(named));
    assert_eq!(graphs[1]["id"], json!(empty_obj));
    assert_eq!(graphs[2]["id"], json!(unnamed));
    assert_eq!(graphs[2]["name"], json!(""));
    assert_eq!(graphs[2]["task_count"], json!(0));
    assert_eq!(graphs[2]["done_count"], json!(0));

    // Rename; a blank name is rejected and leaves the old one.
    let (status, body) = app
        .call(
            Method::PUT,
            &format!("/v0/{unnamed}"),
            Some(json!({ "name": "Launch" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], json!("Launch"));
    let (status, _) = app
        .call(
            Method::PUT,
            &format!("/v0/{unnamed}"),
            Some(json!({ "name": "  " })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, body) = app.call(Method::GET, &format!("/v0/{unnamed}"), None).await;
    assert_eq!(body["name"], json!("Launch"));

    let (status, _) = app.call(Method::GET, "/v0/missing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .call(Method::PUT, "/v0/missing", Some(json!({ "name": "X" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn actors_crud() {
    let app = TestApp::new().await;

    let (status, body) = app.call(Method::GET, "/v0/actors", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!([]));

    let ann = app.add_actor(json!({ "name": "  Ann  " })).await;
    assert_eq!(ann.name, "Ann");
    assert_eq!(ann.kind, ActorKind::Human);
    let bot = app
        .add_actor(json!({ "name": "bot", "kind": "agent" }))
        .await;
    assert_eq!(bot.kind, ActorKind::Agent);

    let (status, body) = app.call(Method::GET, "/v0/actors", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_value::<Vec<Actor>>(body).unwrap(),
        vec![ann.clone(), bot.clone()]
    );
    let (status, body) = app
        .call(Method::GET, &format!("/v0/actors/{}", bot.id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(serde_json::from_value::<Actor>(body).unwrap(), bot);

    // Names are unique, and may not shadow another actor's ID either.
    for name in ["Ann", bot.id.as_str(), " "] {
        let (status, _) = app
            .call(Method::POST, "/v0/actors", Some(json!({ "name": name })))
            .await;
        assert!(
            status == StatusCode::CONFLICT || (name == " " && status == StatusCode::BAD_REQUEST),
            "{name}: {status}"
        );
    }
    let (status, _) = app
        .call(
            Method::POST,
            "/v0/actors",
            Some(json!({ "name": "x", "kind": "robot" })),
        )
        .await;
    assert!(status.is_client_error());

    // Partial edit; keeping one's own name is fine, taking another's is not.
    let uri = format!("/v0/actors/{}", ann.id);
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "kind": "agent" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({ "id": ann.id, "name": "Ann", "kind": "agent", "task_count": 0 })
    );
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "name": "Ann" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = app
        .call(Method::PUT, &uri, Some(json!({ "name": "bot" })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = app
        .call(Method::PUT, &uri, Some(json!({ "name": "" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = app
        .call(
            Method::PUT,
            "/v0/actors/missing",
            Some(json!({ "name": "Z" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app.call(Method::GET, "/v0/actors/missing", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn assignees_must_be_actors() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "Ann" })).await;
    let bot = app
        .add_actor(json!({ "name": "bot", "kind": "agent" }))
        .await;

    // By ID or by name, duplicates collapse.
    let task = app
        .add_task(
            &graph_id,
            json!({ "name": "T", "users": [ann.id, "Ann", "bot"] }),
        )
        .await;
    let mut users = task.users.clone();
    users.sort();
    let mut expected = vec![ann.id.clone(), bot.id.clone()];
    expected.sort();
    assert_eq!(users, expected);

    // An unknown actor is a 404 and changes nothing.
    let (status, body) = app
        .call(
            Method::POST,
            &format!("/v0/{graph_id}/task/add/"),
            Some(json!({ "name": "U", "users": ["nobody"] })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body["message"].as_str().unwrap().contains("nobody"));
    let uri = format!("/v0/{graph_id}/task/{}", task.id);
    let (status, _) = app
        .call(
            Method::PUT,
            &uri,
            Some(json!({ "name": "Renamed", "users": ["nobody"] })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.all_tasks(&graph_id).await, vec![task.clone()]);

    // Renaming an actor keeps its assignments.
    app.call(
        Method::PUT,
        &format!("/v0/actors/{}", ann.id),
        Some(json!({ "name": "Anna" })),
    )
    .await;
    let (status, body) = app
        .call(Method::PUT, &uri, Some(json!({ "users": ["Anna"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["users"], json!([ann.id]));

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn actors_delete_and_merge() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "Ann" })).await;
    let dup = app.add_actor(json!({ "name": "ann" })).await;
    let bob = app.add_actor(json!({ "name": "bob" })).await;
    let both = app
        .add_task(
            &graph_id,
            json!({ "name": "both", "users": [ann.id, dup.id] }),
        )
        .await;
    let only_dup = app
        .add_task(&graph_id, json!({ "name": "dup", "users": [dup.id] }))
        .await;
    let only_bob = app
        .add_task(&graph_id, json!({ "name": "bob", "users": [bob.id] }))
        .await;

    let actor = |id: &str| {
        let uri = format!("/v0/actors/{id}");
        let app = &app;
        async move { app.call(Method::GET, &uri, None).await }
    };
    let (_, body) = actor(&dup.id).await;
    assert_eq!(body["task_count"], json!(2));

    // Merging into itself or into nobody changes nothing.
    let merge = format!("/v0/actors/{}/merge", dup.id);
    let (status, _) = app
        .call(Method::POST, &merge, Some(json!({ "into": "ann" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = app
        .call(Method::POST, &merge, Some(json!({ "into": "nobody" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .call(
            Method::POST,
            "/v0/actors/missing/merge",
            Some(json!({ "into": "Ann" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // The duplicate's tasks move over by name, a shared task keeps one assignee.
    let (status, body) = app
        .call(Method::POST, &merge, Some(json!({ "into": "Ann" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], json!(ann.id));
    assert_eq!(body["task_count"], json!(2));
    assert_eq!(actor(&dup.id).await.0, StatusCode::NOT_FOUND);
    let tasks = app.all_tasks(&graph_id).await;
    let users = |id: &str| tasks.iter().find(|t| t.id == id).unwrap().users.clone();
    assert_eq!(users(&both.id), vec![ann.id.clone()]);
    assert_eq!(users(&only_dup.id), vec![ann.id.clone()]);
    assert_eq!(users(&only_bob.id), vec![bob.id.clone()]);

    // An assigned actor is only deleted when asked to drop its assignments.
    let uri = format!("/v0/actors/{}", bob.id);
    let (status, body) = app.call(Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(actor(&bob.id).await.0, StatusCode::OK);
    let (status, body) = app
        .call(Method::DELETE, &format!("{uri}?unassign=true"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(actor(&bob.id).await.0, StatusCode::NOT_FOUND);
    let tasks = app.all_tasks(&graph_id).await;
    assert!(tasks
        .iter()
        .find(|t| t.id == only_bob.id)
        .unwrap()
        .users
        .is_empty());
    assert_eq!(
        app.call(Method::DELETE, &uri, None).await.0,
        StatusCode::NOT_FOUND
    );

    // An unassigned actor goes without asking.
    let free = app.add_actor(json!({ "name": "free" })).await;
    let (status, _) = app
        .call(Method::DELETE, &format!("/v0/actors/{}", free.id), None)
        .await;
    assert_eq!(status, StatusCode::OK);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn bearer_tokens_identify_actors() {
    let mut app = TestApp::new().await;
    let ann = app.add_actor(json!({ "name": "Ann" })).await;
    let dup = app.add_actor(json!({ "name": "ann" })).await;
    let bob = app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, ann_token_id, issued_for) = auth::issue_token(&mut app.graph, "Ann", "laptop")
        .await
        .unwrap();
    assert_eq!(issued_for.id, ann.id);
    let (dup_token, _, _) = auth::issue_token(&mut app.graph, &dup.id, "")
        .await
        .unwrap();
    let (bob_token, _, _) = auth::issue_token(&mut app.graph, "bob", "").await.unwrap();
    assert!(auth::issue_token(&mut app.graph, "nobody", "")
        .await
        .is_err());

    // Who am I: the token's actor; nobody without a token.
    let (status, body) = app
        .call_as(Some(&ann_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["id"], json!(ann.id));
    let (status, _) = app.call(Method::GET, "/v0/me", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Requests without a token are still served; a bad token is refused everywhere.
    assert_eq!(
        app.call(Method::GET, "/v0/graphs", None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        app.call_as(Some(&ann_token), Method::GET, "/v0/graphs", None)
            .await
            .0,
        StatusCode::OK
    );
    for bad in ["mw_0000", "not-a-token", ""] {
        let (status, _) = app
            .call_as(Some(bad), Method::GET, "/v0/graphs", None)
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{bad}");
    }
    // The usual mistake, a token's id instead of the token, gets a message saying so.
    let (status, body) = app
        .call_as(Some(&ann_token_id), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], auth::NOT_A_TOKEN);
    assert_eq!(
        app.call(Method::GET, "/health", None).await.0,
        StatusCode::OK
    );

    let tokens = auth::list_tokens(&mut app.graph, Some("Ann"))
        .await
        .unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(
        (tokens[0].id.as_str(), tokens[0].label.as_str()),
        (ann_token_id.as_str(), "laptop")
    );
    assert_eq!(
        auth::list_tokens(&mut app.graph, None).await.unwrap().len(),
        3
    );

    // A merged duplicate's token now acts as the actor it was merged into.
    let (status, _) = app
        .call(
            Method::POST,
            &format!("/v0/actors/{}/merge", dup.id),
            Some(json!({ "into": ann.id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = app
        .call_as(Some(&dup_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(body["id"], json!(ann.id));

    // Revoked and deleted actors' tokens stop working.
    auth::revoke_token(&mut app.graph, &ann_token_id)
        .await
        .unwrap();
    assert!(auth::revoke_token(&mut app.graph, &ann_token_id)
        .await
        .is_err());
    let (status, _) = app
        .call_as(Some(&ann_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    app.call(Method::DELETE, &format!("/v0/actors/{}", bob.id), None)
        .await;
    let (status, _) = app
        .call_as(Some(&bob_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        auth::list_tokens(&mut app.graph, None).await.unwrap().len(),
        1
    );

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn task_reporter() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    let bob = app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();
    let add = format!("/v0/{graph_id}/task/add/");

    // The author is the reporter by default; without a token there is none.
    let (status, body) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &add,
            Some(json!({ "name": "A" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["reporter"], json!(ann.id));
    let task: Task = serde_json::from_value(body).unwrap();
    let anon = app.add_task(&graph_id, json!({ "name": "B" })).await;
    assert_eq!(anon.reporter, None);

    // Named explicitly, by name or ID.
    let (status, body) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &add,
            Some(json!({ "name": "C", "reporter": "bob" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["reporter"], json!(bob.id));
    let (status, _) = app
        .call(
            Method::POST,
            &add,
            Some(json!({ "name": "D", "reporter": "nobody" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Changed, kept by unrelated edits, removed with "".
    let uri = format!("/v0/{graph_id}/task/{}", task.id);
    let (_, body) = app
        .call(Method::PUT, &uri, Some(json!({ "reporter": bob.id })))
        .await;
    assert_eq!(body["reporter"], json!(bob.id));
    let (_, body) = app
        .call(
            Method::PUT,
            &uri,
            Some(json!({ "status": "REVIEW", "reporter": null })),
        )
        .await;
    assert_eq!(body["reporter"], json!(bob.id));
    let (status, _) = app
        .call(Method::PUT, &uri, Some(json!({ "reporter": "nobody" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, body) = app
        .call(Method::PUT, &uri, Some(json!({ "reporter": "" })))
        .await;
    assert_eq!(body["reporter"], Value::Null);

    // Merging moves reported tasks; deleting leaves them without a reporter.
    app.call(Method::PUT, &uri, Some(json!({ "reporter": "bob" })))
        .await;
    let (status, body) = app
        .call(
            Method::POST,
            &format!("/v0/actors/{}/merge", bob.id),
            Some(json!({ "into": "ann" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let reporter_of =
        |tasks: &[Task], id: &str| tasks.iter().find(|t| t.id == id).unwrap().reporter.clone();
    assert_eq!(
        reporter_of(&app.all_tasks(&graph_id).await, &task.id),
        Some(ann.id.clone())
    );
    let (status, body) = app
        .call(Method::DELETE, &format!("/v0/actors/{}", ann.id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reporter_of(&app.all_tasks(&graph_id).await, &task.id), None);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn claim_task_is_atomic() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();
    let (bob_token, _, _) = auth::issue_token(&mut app.graph, "bob", "").await.unwrap();
    let blocker = app.add_task(&graph_id, json!({ "name": "Blocker" })).await;
    let blocked = app
        .add_task(
            &graph_id,
            json!({ "name": "Blocked", "depends_on": [blocker.id] }),
        )
        .await;
    let claim = |id: &str| format!("/v0/{graph_id}/task/{id}/claim");

    // Without a token there is nobody to assign.
    let (status, _) = app.call(Method::POST, &claim(&blocker.id), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Ready and free: ann gets it; bob is too late, and so is ann a second time.
    let (status, body) = app
        .call_as(Some(&ann_token), Method::POST, &claim(&blocker.id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "IN_PROGRESS");
    assert_eq!(body["users"], json!([ann.id]));
    let (status, body) = app
        .call_as(Some(&bob_token), Method::POST, &claim(&blocker.id), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["message"].as_str().unwrap().contains("not TODO"),
        "{body}"
    );

    // Blocked by an unfinished task.
    let (status, body) = app
        .call_as(Some(&bob_token), Method::POST, &claim(&blocked.id), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["message"].as_str().unwrap().contains("Blocker"),
        "{body}"
    );

    // Assigned to someone else, though TODO and ready.
    let theirs = app
        .add_task(&graph_id, json!({ "name": "Theirs", "users": ["bob"] }))
        .await;
    let (status, body) = app
        .call_as(Some(&ann_token), Method::POST, &claim(&theirs.id), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body["message"].as_str().unwrap().contains("someone else"),
        "{body}"
    );
    // ...but its own assignee may claim it.
    let (status, body) = app
        .call_as(Some(&bob_token), Method::POST, &claim(&theirs.id), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "IN_PROGRESS");

    let (status, _) = app
        .call_as(Some(&ann_token), Method::POST, &claim("no-such-task"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A race: many actors claim one task at once, exactly one wins.
    let contested = app
        .add_task(&graph_id, json!({ "name": "Contested" }))
        .await;
    let mut tokens = Vec::new();
    for i in 0..8 {
        app.add_actor(json!({ "name": format!("racer{i}") })).await;
        let (token, _, _) = auth::issue_token(&mut app.graph, &format!("racer{i}"), "")
            .await
            .unwrap();
        tokens.push(token);
    }
    let uri = claim(&contested.id);
    let results = futures::future::join_all(
        tokens
            .iter()
            .map(|t| app.call_as(Some(t), Method::POST, &uri, None)),
    )
    .await;
    let won = results.iter().filter(|(s, _)| *s == StatusCode::OK).count();
    let lost = results
        .iter()
        .filter(|(s, _)| *s == StatusCode::CONFLICT)
        .count();
    assert_eq!((won, lost), (1, 7), "{results:?}");
    let task = app
        .all_tasks(&graph_id)
        .await
        .into_iter()
        .find(|t| t.id == contested.id)
        .unwrap();
    assert_eq!(task.users.len(), 1);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn free_text_users_migrate_to_actors() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let a = app.add_task(&graph_id, json!({ "name": "A" })).await;
    let b = app.add_task(&graph_id, json!({ "name": "B" })).await;
    // The model before actors: assignees were (:User {id: <free text>}).
    app.graph
        .query(
            "MATCH (a:Task {id: $a}), (b:Task {id: $b}) \
             MERGE (andrew:User {id: 'andrew'}) MERGE (claude:User {id: 'Claude'}) \
             MERGE (a)-[:ASSIGNED_TO]->(andrew) MERGE (a)-[:ASSIGNED_TO]->(claude) \
             MERGE (b)-[:ASSIGNED_TO]->(claude)",
        )
        .with_param("a", a.id.as_str())
        .with_param("b", b.id.as_str())
        .execute()
        .await
        .unwrap();

    // Twice: the second run must be a no-op.
    connection::migrate(&mut app.graph).await.unwrap();
    connection::migrate(&mut app.graph).await.unwrap();

    let (status, body) = app.call(Method::GET, "/v0/actors", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!([
            { "id": "andrew", "name": "andrew", "kind": "human", "task_count": 1 },
            { "id": "Claude", "name": "Claude", "kind": "agent", "task_count": 2 },
        ])
    );
    let tasks = app.all_tasks(&graph_id).await;
    let mut a_users = tasks.iter().find(|t| t.id == a.id).unwrap().users.clone();
    a_users.sort();
    assert_eq!(a_users, vec!["Claude".to_string(), "andrew".to_string()]);
    assert_eq!(
        tasks.iter().find(|t| t.id == b.id).unwrap().users,
        vec!["Claude".to_string()]
    );

    let mut result = app
        .graph
        .query("MATCH (u:User) RETURN count(u) AS n")
        .execute()
        .await
        .unwrap();
    let row = futures::StreamExt::next(&mut result.data)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("n").unwrap(), 0);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn task_comments() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    let bob = app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();
    let (bob_token, _, _) = auth::issue_token(&mut app.graph, "bob", "").await.unwrap();
    let task = app
        .add_task(&graph_id, json!({ "name": "Talk about me" }))
        .await;
    let uri = format!("/v0/{graph_id}/task/{}/comments", task.id);

    let (status, body) = app.call(Method::GET, &uri, None).await;
    assert_eq!((status, body), (StatusCode::OK, json!([])));

    // The author is the token's actor; no token, no comment.
    let (status, _) = app
        .call(Method::POST, &uri, Some(json!({ "body": "hi" })))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    for bad in ["", "   ", &"x".repeat(20_001)] {
        let (status, _) = app
            .call_as(
                Some(&ann_token),
                Method::POST,
                &uri,
                Some(json!({ "body": bad })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, first) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &uri,
            Some(json!({ "body": "  Started, see **PR** \n" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["body"], "Started, see **PR**");
    assert_eq!(first["author"], json!(ann.id));
    assert!(first["created_at"].as_i64().unwrap() > 0);
    let (status, _) = app
        .call_as(
            Some(&bob_token),
            Method::POST,
            &uri,
            Some(json!({ "body": "Thanks" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    // Oldest first.
    let (_, body) = app.call(Method::GET, &uri, None).await;
    let authors: Vec<&Value> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["author"])
        .collect();
    assert_eq!(authors, vec![&json!(ann.id), &json!(bob.id)]);

    // Unknown task, or a task of another graph: 404.
    let (status, _) = app
        .call(
            Method::GET,
            &format!("/v0/{graph_id}/task/nope/comments"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let other = app.create_graph().await;
    let (status, _) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &format!("/v0/{other}/task/{}/comments", task.id),
            Some(json!({ "body": "wrong graph" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Merging moves authorship; deleting the author keeps the comment without one.
    app.call(
        Method::POST,
        &format!("/v0/actors/{}/merge", bob.id),
        Some(json!({ "into": "ann" })),
    )
    .await;
    let (_, body) = app.call(Method::GET, &uri, None).await;
    assert_eq!(body[1]["author"], json!(ann.id));
    let (status, _) = app
        .call(Method::DELETE, &format!("/v0/actors/{}", ann.id), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, body) = app.call(Method::GET, &uri, None).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(body[0]["author"], Value::Null);

    // Deleting the task deletes its comments.
    app.call(
        Method::DELETE,
        &format!("/v0/{graph_id}/task/{}", task.id),
        None,
    )
    .await;
    let mut result = app
        .graph
        .query("MATCH (c:Comment) RETURN count(c) AS n")
        .execute()
        .await
        .unwrap();
    let row = futures::StreamExt::next(&mut result.data)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("n").unwrap(), 0);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn own_tokens_self_service() {
    let mut app = TestApp::new().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, ann_token_id, _) = auth::issue_token(&mut app.graph, "ann", "cli")
        .await
        .unwrap();
    let (_, bob_token_id, _) = auth::issue_token(&mut app.graph, "bob", "").await.unwrap();

    // Nobody without a token: there is no way to get a first token over HTTP.
    for (method, uri) in [
        (Method::GET, "/v0/me/tokens".to_string()),
        (Method::POST, "/v0/me/tokens".to_string()),
        (Method::DELETE, format!("/v0/me/tokens/{ann_token_id}")),
    ] {
        let body = (method == Method::POST).then(|| json!({}));
        let (status, _) = app.call(method, &uri, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // Only the caller's tokens, the one in use marked.
    let (status, body) = app
        .call_as(Some(&ann_token), Method::GET, "/v0/me/tokens", None)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], json!(ann_token_id));
    assert_eq!(body[0]["label"], "cli");
    assert_eq!(body[0]["current"], true);

    // Another token for oneself, shown once, and it works.
    let (status, issued) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            "/v0/me/tokens",
            Some(json!({ "label": " phone " })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{issued}");
    let new_token = issued["token"].as_str().unwrap().to_string();
    assert!(new_token.starts_with("mw_"));
    assert_eq!(issued["info"]["label"], "phone");
    assert_eq!(issued["info"]["actor_id"], json!(ann.id));
    let (status, body) = app
        .call_as(Some(&new_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!((status, &body["id"]), (StatusCode::OK, &json!(ann.id)));
    let (status, _) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            "/v0/me/tokens",
            Some(json!({ "label": "x".repeat(101) })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Someone else's token cannot be revoked, or even seen to exist.
    let (status, _) = app
        .call_as(
            Some(&ann_token),
            Method::DELETE,
            &format!("/v0/me/tokens/{bob_token_id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Revoking one's own: the old token stops working, the new one keeps going.
    let (status, _) = app
        .call_as(
            Some(&new_token),
            Method::DELETE,
            &format!("/v0/me/tokens/{ann_token_id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .call_as(Some(&ann_token), Method::GET, "/v0/me", None)
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, body) = app
        .call_as(Some(&new_token), Method::GET, "/v0/me/tokens", None)
        .await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["current"], true);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn timestamps_and_creators() {
    let mut app = TestApp::new().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();
    let pause = || tokio::time::sleep(std::time::Duration::from_millis(5));

    // Created with a token: the creator is recorded, on the graph and on the task.
    let (status, body) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            "/v0/create_graph",
            Some(json!({ "name": "G" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let graph_id = body["ID"].as_str().unwrap().to_string();
    let (status, body) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &format!("/v0/{graph_id}/task/add/"),
            Some(json!({ "name": "A" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let a: Task = serde_json::from_value(body).unwrap();
    assert_eq!(a.created_by.as_deref(), Some(ann.id.as_str()));
    let created = a.created_at.unwrap();
    assert_eq!(a.updated_at, Some(created));
    let (_, g) = app
        .call(Method::GET, &format!("/v0/{graph_id}"), None)
        .await;
    assert_eq!(g["created_by"], json!(ann.id));

    // Without a token: no creator.
    let b = app.add_task(&graph_id, json!({ "name": "B" })).await;
    assert_eq!(b.created_by, None);
    assert!(b.created_at.is_some());

    // Edits, claims and edge changes move updated_at; created_at stays.
    let get = |id: String| {
        let app = &app;
        let uri = format!("/v0/{graph_id}/task/{id}");
        async move {
            let (_, body) = app.call(Method::GET, &uri, None).await;
            serde_json::from_value::<Task>(body).unwrap()
        }
    };
    pause().await;
    app.call(
        Method::PUT,
        &format!("/v0/{graph_id}/task/{}", a.id),
        Some(json!({ "name": "A2" })),
    )
    .await;
    let edited = get(a.id.clone()).await;
    assert_eq!(edited.created_at, Some(created));
    assert!(edited.updated_at.unwrap() > created);

    pause().await;
    app.call(
        Method::POST,
        &format!("/v0/{graph_id}/connect_nodes/"),
        Some(json!({ "node_dependency": a.id, "node_dependee": b.id })),
    )
    .await;
    let linked = get(b.id.clone()).await;
    assert!(linked.updated_at > b.updated_at);

    pause().await;
    app.call_as(
        Some(&ann_token),
        Method::POST,
        &format!("/v0/{graph_id}/task/{}/claim", a.id),
        None,
    )
    .await;
    let claimed = get(a.id.clone()).await;
    assert!(claimed.updated_at > edited.updated_at);

    // The graph's updated_at follows its latest task change, and deletions.
    let (_, g) = app
        .call(Method::GET, &format!("/v0/{graph_id}"), None)
        .await;
    assert_eq!(g["updated_at"].as_i64(), claimed.updated_at);
    pause().await;
    app.call(
        Method::DELETE,
        &format!("/v0/{graph_id}/task/{}", b.id),
        None,
    )
    .await;
    let (_, g) = app
        .call(Method::GET, &format!("/v0/{graph_id}"), None)
        .await;
    assert!(g["updated_at"].as_i64() > claimed.updated_at);

    // Merging moves what the actor created; deleting leaves it without a creator.
    app.call(
        Method::POST,
        &format!("/v0/actors/{}/merge", ann.id),
        Some(json!({ "into": "bob" })),
    )
    .await;
    let bob_id = app
        .call(Method::GET, "/v0/actors", None)
        .await
        .1
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "bob")
        .unwrap()["id"]
        .clone();
    assert_eq!(json!(get(a.id.clone()).await.created_by), bob_id);
    let (_, g) = app
        .call(Method::GET, &format!("/v0/{graph_id}"), None)
        .await;
    assert_eq!(g["created_by"], bob_id);
    app.call(
        Method::DELETE,
        &format!("/v0/actors/{}?unassign=true", bob_id.as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(get(a.id.clone()).await.created_by, None);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn layout_positions_persist() {
    let app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let a = app.add_task(&graph_id, json!({ "name": "A" })).await;
    let b = app.add_task(&graph_id, json!({ "name": "B" })).await;
    assert_eq!(a.position, None);
    let layout = format!("/v0/{graph_id}/layout");
    let position_of = |tasks: &[Task], id: &str| {
        tasks
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .position
            .map(|p| (p.x, p.y))
    };

    let (status, body) = app
        .call(
            Method::PUT,
            &layout,
            Some(json!({ "positions": {
                (a.id.clone()): { "x": 10.5, "y": -20 },
                (b.id.clone()): { "x": 300, "y": 40 },
            } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let tasks = app.all_tasks(&graph_id).await;
    assert_eq!(position_of(&tasks, &a.id), Some((10.5, -20.0)));
    assert_eq!(position_of(&tasks, &b.id), Some((300.0, 40.0)));
    // Layout is not a change to the task.
    assert_eq!(
        tasks.iter().find(|t| t.id == a.id).unwrap().updated_at,
        a.updated_at
    );

    // Partial: only B moves.
    app.call(
        Method::PUT,
        &layout,
        Some(json!({ "positions": { (b.id.clone()): { "x": 0, "y": 0 } } })),
    )
    .await;
    let tasks = app.all_tasks(&graph_id).await;
    assert_eq!(position_of(&tasks, &a.id), Some((10.5, -20.0)));
    assert_eq!(position_of(&tasks, &b.id), Some((0.0, 0.0)));

    // A task of another graph: 404 and nothing saved.
    let other = app.create_graph().await;
    let stranger = app.add_task(&other, json!({ "name": "S" })).await;
    let (status, _) = app
        .call(
            Method::PUT,
            &layout,
            Some(json!({ "positions": {
                (a.id.clone()): { "x": 1, "y": 1 },
                (stranger.id.clone()): { "x": 1, "y": 1 },
            } })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let tasks = app.all_tasks(&graph_id).await;
    assert_eq!(position_of(&tasks, &a.id), Some((10.5, -20.0)));

    let (status, _) = app
        .call(Method::PUT, &layout, Some(json!({ "positions": {} })))
        .await;
    assert_eq!(status, StatusCode::OK);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn change_log_events() {
    let mut app = TestApp::new().await;
    let graph_id = app.create_graph().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    app.add_actor(json!({ "name": "bob" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();
    let blocker = app.add_task(&graph_id, json!({ "name": "Blocker" })).await;
    let (_, body) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            &format!("/v0/{graph_id}/task/add/"),
            Some(json!({ "name": "T" })),
        )
        .await;
    let task: Task = serde_json::from_value(body).unwrap();
    let uri = format!("/v0/{graph_id}/task/{}", task.id);
    let events = || {
        let app = &app;
        let uri = format!("{uri}/events");
        async move {
            let (status, body) = app.call(Method::GET, &uri, None).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            body.as_array().unwrap().clone()
        }
    };

    // Created, by the token's actor.
    let log = events().await;
    assert_eq!(log.len(), 1);
    assert_eq!(
        (&log[0]["kind"], &log[0]["actor"]),
        (&json!("created"), &json!(ann.id))
    );

    // One PUT, several fields: one event each; unchanged fields log nothing; the
    // description is logged as changed without its text.
    app.call_as(
        Some(&ann_token),
        Method::PUT,
        &uri,
        Some(json!({
            "name": "T2", "status": "IN_PROGRESS", "users": ["bob"],
            "description": "secret plan", "tags": [], "depends_on": [blocker.id]
        })),
    )
    .await;
    let log = events().await;
    let summary: Vec<(String, String)> = log[1..]
        .iter()
        .map(|e| {
            (
                e["kind"].as_str().unwrap().to_string(),
                e["field"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("field", "name"),
            ("field", "description"),
            ("field", "status"),
            ("field", "users"),
            ("blocker_added", ""),
        ]
        .map(|(k, f)| (k.to_string(), f.to_string()))
    );
    assert_eq!(
        (&log[3]["from"], &log[3]["to"]),
        (&json!("TODO"), &json!("IN_PROGRESS"))
    );
    assert_eq!(log[2]["to"], json!("changed"));
    assert!(!log[2].to_string().contains("secret plan"));
    assert_eq!(log[5]["to"], json!(blocker.id));

    // Edge endpoints and an anonymous edit: logged without an actor.
    app.call(
        Method::POST,
        &format!("/v0/{graph_id}/disconnect_nodes/"),
        Some(json!({ "node_dependency": blocker.id, "node_dependee": task.id })),
    )
    .await;
    let log = events().await;
    let last = log.last().unwrap();
    assert_eq!(
        (&last["kind"], &last["actor"]),
        (&json!("blocker_removed"), &Value::Null)
    );

    // A no-op PUT logs nothing.
    let before = events().await.len();
    app.call(Method::PUT, &uri, Some(json!({ "name": "T2" })))
        .await;
    assert_eq!(events().await.len(), before);

    // Claim logs status and assignee.
    let free = app.add_task(&graph_id, json!({ "name": "Free" })).await;
    app.call_as(
        Some(&ann_token),
        Method::POST,
        &format!("/v0/{graph_id}/task/{}/claim", free.id),
        None,
    )
    .await;
    let (_, log) = app
        .call(
            Method::GET,
            &format!("/v0/{graph_id}/task/{}/events", free.id),
            None,
        )
        .await;
    let fields: Vec<&str> = log.as_array().unwrap()[1..]
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["status", "users"]);

    // Deleting the task deletes its log.
    app.call(Method::DELETE, &uri, None).await;
    let mut result = app
        .graph
        .query("MATCH (e:Event)-[:ON]->(:Task {id: $id}) RETURN count(e) AS n")
        .with_param("id", task.id.as_str())
        .execute()
        .await
        .unwrap();
    let row = futures::StreamExt::next(&mut result.data)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("n").unwrap(), 0);
    let mut result = app
        .graph
        .query("MATCH (e:Event) WHERE NOT (e)-[:ON]->() RETURN count(e) AS n")
        .execute()
        .await
        .unwrap();
    let row = futures::StreamExt::next(&mut result.data)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("n").unwrap(), 0, "orphaned events");

    app.cleanup().await;
}

/// Runs a `RETURN count(...) AS n` query straight against FalkorDB.
async fn count(graph: &mut AsyncGraph, q: &str, id: &str) -> i64 {
    let mut result = graph.query(q).with_param("id", id).execute().await.unwrap();
    let row = futures::StreamExt::next(&mut result.data)
        .await
        .unwrap()
        .unwrap();
    row.try_get::<i64>("n").unwrap()
}

fn graph_ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn spaces_hold_graphs() {
    let mut app = TestApp::new().await;
    let ann = app.add_actor(json!({ "name": "ann" })).await;
    let (ann_token, _, _) = auth::issue_token(&mut app.graph, "ann", "").await.unwrap();

    // A graph created without a space lands in the default space, made on the way.
    let (status, body) = app
        .call(
            Method::POST,
            "/v0/create_graph",
            Some(json!({ "name": "Old" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["space"], "default");
    let old = body["ID"].as_str().unwrap().to_string();

    // Creating a space: 201, with the creator; a blank name is a 400.
    let (status, _) = app
        .call(Method::POST, "/v0/spaces", Some(json!({ "name": "  " })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, work) = app
        .call_as(
            Some(&ann_token),
            Method::POST,
            "/v0/spaces",
            Some(json!({ "name": " Work " })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{work}");
    assert_eq!(work["name"], "Work");
    assert_eq!(work["graph_count"], 0);
    assert_eq!(work["created_by"], ann.id.as_str());
    assert!(work["created_at"].is_i64());
    let work_id = work["id"].as_str().unwrap().to_string();

    // A graph in that space; an unknown space is a 404 and creates nothing.
    let (status, body) = app
        .call(
            Method::POST,
            "/v0/create_graph",
            Some(json!({ "name": "New", "space": work_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["space"], work_id.as_str());
    let new = body["ID"].as_str().unwrap().to_string();
    let (status, _) = app
        .call(
            Method::POST,
            "/v0/create_graph",
            Some(json!({ "name": "Lost", "space": "nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Listing: every graph carries its space; ?space= and /spaces/:id/graphs filter.
    let (status, all) = app.call(Method::GET, "/v0/graphs", None).await;
    assert_eq!(status, StatusCode::OK, "{all}");
    assert_eq!(all.as_array().unwrap().len(), 2);
    for g in all.as_array().unwrap() {
        let expected = if g["id"] == old.as_str() {
            "default"
        } else {
            work_id.as_str()
        };
        assert_eq!(g["space"], expected, "{g}");
    }
    let (_, in_work) = app
        .call(Method::GET, &format!("/v0/graphs?space={work_id}"), None)
        .await;
    assert_eq!(graph_ids(&in_work), vec![new.clone()]);
    let (_, in_default) = app
        .call(Method::GET, "/v0/spaces/default/graphs", None)
        .await;
    assert_eq!(graph_ids(&in_default), vec![old.clone()]);
    let (status, _) = app.call(Method::GET, "/v0/graphs?space=nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app.call(Method::GET, "/v0/spaces/nope/graphs", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Spaces with counters, oldest first.
    let (status, spaces) = app.call(Method::GET, "/v0/spaces", None).await;
    assert_eq!(status, StatusCode::OK, "{spaces}");
    let spaces = spaces.as_array().unwrap();
    assert_eq!(spaces.len(), 2);
    assert_eq!(spaces[0]["id"], "default");
    assert_eq!(spaces[0]["name"], "Default");
    assert_eq!(spaces[0]["graph_count"], 1);
    assert_eq!(spaces[1]["id"], work_id.as_str());
    assert_eq!(spaces[1]["graph_count"], 1);

    // Rename; blank → 400, unknown → 404.
    let (status, body) = app
        .call(
            Method::PUT,
            &format!("/v0/spaces/{work_id}"),
            Some(json!({ "name": "Job" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Job");
    let (_, body) = app
        .call(Method::GET, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(body["name"], "Job");
    let (status, _) = app
        .call(
            Method::PUT,
            &format!("/v0/spaces/{work_id}"),
            Some(json!({ "name": "" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = app
        .call(Method::PUT, "/v0/spaces/nope", Some(json!({ "name": "X" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A space with graphs cannot be deleted.
    let (status, _) = app
        .call(Method::DELETE, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Moving a graph: an unknown space is a 404 and nothing changes; then the move.
    let (status, _) = app
        .call(
            Method::PUT,
            &format!("/v0/{new}"),
            Some(json!({ "name": "Renamed", "space": "nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, body) = app.call(Method::GET, &format!("/v0/{new}"), None).await;
    assert_eq!(body["name"], "New");
    assert_eq!(body["space"], work_id.as_str());
    let (status, body) = app
        .call(
            Method::PUT,
            &format!("/v0/{new}"),
            Some(json!({ "space": "default" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["space"], "default");
    assert_eq!(body["name"], "New");
    let (_, body) = app
        .call(Method::GET, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(body["graph_count"], 0);
    let (_, body) = app.call(Method::GET, "/v0/spaces/default", None).await;
    assert_eq!(body["graph_count"], 2);
    let edges = count(
        &mut app.graph,
        "MATCH (:Graph {id: $id})-[r:IN]->() RETURN count(r) AS n",
        &new,
    )
    .await;
    assert_eq!(edges, 1);

    // Now empty, it can go; the default space never can.
    let (status, _) = app
        .call(Method::DELETE, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .call(Method::GET, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .call(Method::DELETE, &format!("/v0/spaces/{work_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app.call(Method::DELETE, "/v0/spaces/default", None).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Task routes do not care about spaces.
    app.add_task(&new, json!({ "name": "Still works" })).await;
    assert_eq!(app.all_tasks(&new).await.len(), 1);

    app.cleanup().await;
}

#[tokio::test]
#[ignore = "needs FalkorDB"]
async fn graphs_migrate_into_default_space() {
    let mut app = TestApp::new().await;
    // Graphs from before spaces: no IN edge, no Space node at all.
    app.graph
        .query(
            "CREATE (:Graph {id: 'g1', name: 'One', created_at: 1}), \
                    (:Graph {id: 'g2', name: 'Two'})",
        )
        .execute()
        .await
        .unwrap();

    // Twice: the second run must be a no-op.
    connection::migrate(&mut app.graph).await.unwrap();
    connection::migrate(&mut app.graph).await.unwrap();

    let (status, spaces) = app.call(Method::GET, "/v0/spaces", None).await;
    assert_eq!(status, StatusCode::OK, "{spaces}");
    assert_eq!(spaces.as_array().unwrap().len(), 1);
    assert_eq!(spaces[0]["id"], "default");
    assert_eq!(spaces[0]["name"], "Default");
    assert_eq!(spaces[0]["graph_count"], 2);
    let (_, graphs) = app.call(Method::GET, "/v0/graphs", None).await;
    assert_eq!(graphs.as_array().unwrap().len(), 2);
    for g in graphs.as_array().unwrap() {
        assert_eq!(g["space"], "default", "{g}");
    }
    let edges = count(
        &mut app.graph,
        "MATCH (g:Graph)-[r:IN]->(:Space) WHERE g.id STARTS WITH $id RETURN count(r) AS n",
        "g",
    )
    .await;
    assert_eq!(edges, 2);

    // A renamed default space keeps its name when the migration runs again.
    let (status, _) = app
        .call(
            Method::PUT,
            "/v0/spaces/default",
            Some(json!({ "name": "Home" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    connection::migrate(&mut app.graph).await.unwrap();
    let (_, body) = app.call(Method::GET, "/v0/spaces/default", None).await;
    assert_eq!(body["name"], "Home");

    app.cleanup().await;
}
