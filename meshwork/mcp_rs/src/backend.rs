//! Thin REST client for the Meshwork backend (`/v0/...`).

use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;

use crate::model::{NewTask, Task, TaskUpdate};

#[derive(Clone)]
pub struct Backend {
    client: Client,
    base: String,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
}

#[derive(Deserialize)]
struct CreatedGraph {
    #[serde(rename = "ID")]
    id: String,
}

impl Backend {
    pub fn new(base: &str) -> Self {
        Self {
            client: Client::new(),
            base: base.trim_end_matches('/').to_string(),
        }
    }

    fn req(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("{}/v0/{}", self.base, path))
    }

    /// Sends the request and decodes the body; backend `{message}` errors become the `Err` text.
    async fn send<T: DeserializeOwned>(&self, req: RequestBuilder) -> Result<T, String> {
        let resp = req
            .send()
            .await
            .map_err(|e| format!("backend unreachable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json()
                .await
                .map_err(|e| format!("bad backend response: {e}"));
        }
        let text = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<ErrorBody>(&text)
            .map(|b| b.message)
            .unwrap_or(text);
        Err(format!("{status}: {message}"))
    }

    pub async fn create_graph(&self) -> Result<String, String> {
        let g: CreatedGraph = self.send(self.req(Method::POST, "create_graph")).await?;
        Ok(g.id)
    }

    pub async fn all_tasks(&self, graph: &str) -> Result<Vec<Task>, String> {
        self.send(self.req(Method::GET, &format!("{graph}/all_tasks")))
            .await
    }

    pub async fn get_task(&self, graph: &str, id: &str) -> Result<Task, String> {
        self.send(self.req(Method::GET, &format!("{graph}/task/{id}")))
            .await
    }

    pub async fn add_task(&self, graph: &str, task: &NewTask) -> Result<Task, String> {
        self.send(
            self.req(Method::POST, &format!("{graph}/task/add/"))
                .json(task),
        )
        .await
    }

    pub async fn update_task(
        &self,
        graph: &str,
        id: &str,
        update: &TaskUpdate,
    ) -> Result<Task, String> {
        self.send(
            self.req(Method::PUT, &format!("{graph}/task/{id}"))
                .json(update),
        )
        .await
    }

    pub async fn connect(&self, graph: &str, blocker: &str, blocked: &str) -> Result<(), String> {
        self.edge(graph, "connect_nodes/", blocker, blocked).await
    }

    pub async fn disconnect(
        &self,
        graph: &str,
        blocker: &str,
        blocked: &str,
    ) -> Result<(), String> {
        self.edge(graph, "disconnect_nodes/", blocker, blocked)
            .await
    }

    async fn edge(
        &self,
        graph: &str,
        op: &str,
        blocker: &str,
        blocked: &str,
    ) -> Result<(), String> {
        let body = json!({ "node_dependency": blocker, "node_dependee": blocked });
        let _: serde_json::Value = self
            .send(self.req(Method::POST, &format!("{graph}/{op}")).json(&body))
            .await?;
        Ok(())
    }
}
