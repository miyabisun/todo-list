//! MCP tools over the same store as the HTTP API. No caller authentication:
//! the service relies on its LAN/Tailscale boundary like the browser UI.
use crate::{Error, store::Store, todo::Todo};
use axum::Router;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;

const INSTRUCTIONS: &str = "A list of things the human user must do by hand. \
When the user asks to add something to their ToDo, call todo_add with a short title, \
a url the user can tap to act (http/https only), an optional due date (YYYY-MM-DD), \
source naming you or your app, and a body memo with any details. \
Use todo_list/todo_get to read, todo_update to edit, todo_complete when it is done.";

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Id {
    pub id: String,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct List {
    /// true lists completed items (newest first); omitted or false lists open items.
    #[serde(default)]
    pub done: bool,
    /// 1..200, default 50.
    pub limit: Option<usize>,
    /// Default 0; follow `next_offset` until it is null.
    pub offset: Option<usize>,
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Add {
    /// One short line saying what the user has to do.
    pub title: String,
    /// http(s) link that opens where the user acts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Due date as YYYY-MM-DD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    /// Who registered it, e.g. the agent or app name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Optional Markdown memo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(d).map(Some)
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Update {
    pub id: String,
    pub title: Option<String>,
    /// null clears the link.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "Option<String>")]
    pub url: Option<Value>,
    /// YYYY-MM-DD; null clears the due date.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "Option<String>")]
    pub due: Option<Value>,
    /// null clears the source.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "Option<String>")]
    pub source: Option<Value>,
    /// Replaces the whole memo.
    pub body: Option<String>,
    /// false reopens a completed item.
    pub done: Option<bool>,
}

impl Update {
    fn fields(self) -> Value {
        let mut fields = Map::new();
        for (key, value) in [
            ("title", self.title.map(Value::from)),
            ("url", self.url),
            ("due", self.due),
            ("source", self.source),
            ("body", self.body.map(Value::from)),
            ("done", self.done.map(Value::from)),
        ] {
            if let Some(value) = value {
                fields.insert(key.into(), value);
            }
        }
        Value::Object(fields)
    }
}

fn answer(result: Result<Value, Error>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::structured(value),
        Err(e) => {
            let mut r =
                CallToolResult::structured(json!({"error": e.to_string(), "code": e.code()}));
            r.is_error = Some(true);
            r
        }
    }
}

fn full(todo: &Todo) -> Value {
    serde_json::to_value(todo).expect("ToDo serializes")
}

/// List entries leave out the memo and timestamps other than completion.
fn brief(todo: &Todo) -> Value {
    json!({
        "id": todo.id, "title": todo.meta.title, "url": todo.meta.url,
        "due": todo.meta.due, "source": todo.meta.source, "done_at": todo.meta.done_at,
    })
}

#[derive(Clone)]
struct Tools {
    store: Arc<Store>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl Tools {
    fn new(store: Arc<Store>) -> Self {
        Self {
            store,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List ToDo summaries without memos. Open items sort by due date (undated last) then age; done:true lists completed items newest first. Returns {todos,total,next_offset}."
    )]
    fn todo_list(&self, Parameters(a): Parameters<List>) -> CallToolResult {
        answer((|| {
            let limit = a.limit.unwrap_or(50);
            if !(1..=200).contains(&limit) {
                return Err(Error::Invalid("limit must be 1..200".into()));
            }
            let todos = self.store.list(Some(a.done))?;
            let total = todos.len();
            let offset = a.offset.unwrap_or(0).min(total);
            let end = offset.saturating_add(limit).min(total);
            Ok(json!({
                "todos": todos[offset..end].iter().map(brief).collect::<Vec<_>>(),
                "total": total,
                "next_offset": (end < total).then_some(end),
            }))
        })())
    }

    #[tool(description = "Read one ToDo including its memo (body) and timestamps.")]
    fn todo_get(&self, Parameters(a): Parameters<Id>) -> CallToolResult {
        answer(self.store.get(&a.id).map(|t| full(&t)))
    }

    #[tool(
        description = "Add a ToDo for the user. Only title is required. The server assigns the id; returns the saved item."
    )]
    fn todo_add(&self, Parameters(a): Parameters<Add>) -> CallToolResult {
        let fields = serde_json::to_value(&a).expect("arguments serialize");
        answer(self.store.create(&fields).map(|t| full(&t)))
    }

    #[tool(
        description = "Change fields of a ToDo. Omitted fields stay; url/due/source null clears; body replaces the memo; done:false reopens."
    )]
    fn todo_update(&self, Parameters(a): Parameters<Update>) -> CallToolResult {
        let id = a.id.clone();
        answer(self.store.update(&id, &a.fields()).map(|t| full(&t)))
    }

    #[tool(description = "Mark a ToDo done. Repeating keeps the first completion time.")]
    fn todo_complete(&self, Parameters(a): Parameters<Id>) -> CallToolResult {
        answer(
            self.store
                .update(&a.id, &json!({"done": true}))
                .map(|t| full(&t)),
        )
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Tools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(INSTRUCTIONS)
    }
}

pub fn endpoint<S: Clone + Send + Sync + 'static>(store: Arc<Store>) -> Router<S> {
    let tools = Tools::new(store);
    let service = StreamableHttpService::new(
        move || Ok(tools.clone()),
        Arc::new(LocalSessionManager::default()),
        // Callers reach the service by LAN or tailnet host names.
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    Router::new().nest_service("/mcp", service)
}
