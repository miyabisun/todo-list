//! HTTP and MCP share one Markdown directory; every test uses its own temp dir.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
};
use serde_json::{Value, json};
use todo_list::{app, store::Store};
use tower::ServiceExt;

fn server() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("todos")).unwrap();
    (dir, app(store))
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header("content-type", "application/json");
    }
    let body = body.map_or_else(Body::empty, |v| Body::from(v.to_string()));
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn http_adds_lists_completes_and_reopens_markdown_todos() {
    let (dir, app) = server();
    let (status, created) = send(
        &app,
        "POST",
        "/api/todos",
        Some(json!({"title":"架空の申請書を出す","url":"https://example.com/form","due":"2026-10-01","source":"example-agent","body":"印鑑を持つ"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = created["id"].as_str().unwrap().to_owned();
    assert!(created["done_at"].is_null());
    assert_eq!(created["created_at"], created["updated_at"]);

    let file = std::fs::read_to_string(dir.path().join("todos").join(format!("{id}.md"))).unwrap();
    assert!(file.starts_with("---\n"), "{file}");
    assert!(file.contains("title: 架空の申請書を出す\n"), "{file}");
    assert!(file.ends_with("---\n印鑑を持つ"), "{file}");

    let (_, second) = send(&app, "POST", "/api/todos", Some(json!({"title":"undated"}))).await;
    let (status, open) = send(&app, "GET", "/api/todos?done=false", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(open.as_array().unwrap().len(), 2);
    assert_eq!(open[0]["id"], id, "dated item sorts first");
    assert_eq!(open[0]["body"], "印鑑を持つ");

    let (status, done) = send(
        &app,
        "PATCH",
        &format!("/api/todos/{id}"),
        Some(json!({"done":true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(done["done_at"].is_string());
    let (_, open) = send(&app, "GET", "/api/todos?done=false", None).await;
    assert_eq!(open.as_array().unwrap().len(), 1);
    let (_, finished) = send(&app, "GET", "/api/todos?done=true", None).await;
    assert_eq!(finished[0]["id"], id);
    let (_, all) = send(&app, "GET", "/api/todos", None).await;
    assert_eq!(all[0]["id"], second["id"], "open items precede done items");
    assert_eq!(all[1]["id"], id);

    let (_, reopened) = send(
        &app,
        "PATCH",
        &format!("/api/todos/{id}"),
        Some(json!({"done":false,"url":null})),
    )
    .await;
    assert!(reopened["done_at"].is_null());
    assert!(reopened["url"].is_null());
    let (_, fetched) = send(&app, "GET", &format!("/api/todos/{id}"), None).await;
    assert_eq!(fetched, reopened);

    let (status, _) = send(&app, "DELETE", &format!("/api/todos/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, error) = send(&app, "GET", &format!("/api/todos/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["code"], "not_found");
}

#[tokio::test]
async fn http_rejects_invalid_input_without_writing() {
    let (dir, app) = server();
    for body in [
        json!({}),
        json!({"title":""}),
        json!({"title":"x","url":"javascript:alert(1)"}),
        json!({"title":"x","due":"tomorrow"}),
        json!({"title":"x","id":"chosen"}),
    ] {
        let (status, error) = send(&app, "POST", "/api/todos", Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(error["code"], "invalid", "{body}");
    }
    assert_eq!(
        std::fs::read_dir(dir.path().join("todos")).unwrap().count(),
        0
    );
    let (_, created) = send(&app, "POST", "/api/todos", Some(json!({"title":"keep"}))).await;
    let uri = format!("/api/todos/{}", created["id"].as_str().unwrap());
    let (status, _) = send(
        &app,
        "PATCH",
        &uri,
        Some(json!({"title":"changed","due":"2026-13-01"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(send(&app, "GET", &uri, None).await.1["title"], "keep");
    for uri in ["/api/todos/..%2Fescape", "/api/todos/missing"] {
        assert_eq!(
            send(&app, "GET", uri, None).await.0,
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
    assert_eq!(
        send(&app, "GET", "/api/todos?done=maybe", None).await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn hand_written_files_are_listed_and_keep_unknown_fields() {
    let (dir, app) = server();
    std::fs::write(
        dir.path().join("todos/hand_made.md"),
        "---\ntitle: 手書きの項目\ncreated_at: 2026-01-01T00:00:00Z\nupdated_at: 2026-01-01T00:00:00Z\ncolor: blue\n---\nメモ\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("todos/notes.txt"), "ignored").unwrap();
    let (_, all) = send(&app, "GET", "/api/todos", None).await;
    assert_eq!(all.as_array().unwrap().len(), 1);
    assert_eq!(all[0]["id"], "hand_made");
    assert_eq!(all[0]["color"], "blue");
    send(
        &app,
        "PATCH",
        "/api/todos/hand_made",
        Some(json!({"done":true})),
    )
    .await;
    let file = std::fs::read_to_string(dir.path().join("todos/hand_made.md")).unwrap();
    assert!(file.contains("color: blue\n"), "{file}");
    assert!(file.ends_with("---\nメモ\n"), "{file}");
}

#[tokio::test]
async fn snapshot_returns_every_markdown_file_verbatim() {
    let (dir, app) = server();
    let (_, created) = send(
        &app,
        "POST",
        "/api/todos",
        Some(json!({"title":"a","done":true})),
    )
    .await;
    let name = format!("{}.md", created["id"].as_str().unwrap());
    let (status, snapshot) = send(&app, "GET", "/api/snapshot", None).await;
    assert_eq!(status, StatusCode::OK);
    let files = snapshot["files"].as_object().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[&name],
        std::fs::read_to_string(dir.path().join("todos").join(&name)).unwrap()
    );
}

#[tokio::test]
async fn ui_health_and_unknown_api_routes() {
    let (_dir, app) = server();
    let response = app
        .clone()
        .oneshot(Request::get("/somewhere").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/html");
    let response = app
        .clone()
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "ok\n"
    );
    assert_eq!(
        send(&app, "GET", "/api/health", None).await.1,
        json!({"status":"ok"})
    );
    for uri in ["/api", "/api/", "/api/missing"] {
        assert_eq!(
            send(&app, "GET", uri, None).await.0,
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
}

async fn rpc(app: &Router, session: Option<&str>, message: Value) -> (HeaderMap, Value) {
    let mut request = Request::post("/mcp")
        .header("host", "localhost")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream");
    if let Some(session) = session {
        request = request.header("mcp-session-id", session);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(message.to_string())).unwrap())
        .await
        .unwrap();
    let headers = response.headers().clone();
    assert!(response.status().is_success(), "{}", response.status());
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let value = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("data: ")
                .and_then(|j| serde_json::from_str(j).ok())
        })
        .or_else(|| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    (headers, value)
}

async fn call(app: &Router, session: &str, name: &str, arguments: Value) -> Value {
    let (_, reply) = rpc(
        app,
        Some(session),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":name,"arguments":arguments}}),
    )
    .await;
    reply["result"].clone()
}

/// Initialize a session and check the advertised tools.
async fn session(app: &Router) -> String {
    let (headers, init) = rpc(
        app,
        None,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
    )
    .await;
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("todo_add")
    );
    let session = headers["mcp-session-id"].to_str().unwrap().to_owned();
    rpc(
        app,
        Some(&session),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    let (_, tools) = rpc(
        app,
        Some(&session),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    let mut names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "todo_add",
            "todo_complete",
            "todo_get",
            "todo_list",
            "todo_update"
        ]
    );

    session
}

#[tokio::test]
async fn mcp_tools_share_the_http_todos() {
    let (_dir, app) = server();
    let session = session(&app).await;
    let added = call(&app, &session, "todo_add", json!({"title":"架空の返却","url":"https://example.org/r","source":"example-agent","body":"memo"})).await;
    let id = added["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        send(&app, "GET", &format!("/api/todos/{id}"), None).await.1["source"],
        "example-agent"
    );
    let (_, http) = send(
        &app,
        "POST",
        "/api/todos",
        Some(json!({"title":"from http","due":"2026-01-01"})),
    )
    .await;

    let listed = call(&app, &session, "todo_list", json!({})).await["structuredContent"].clone();
    assert_eq!(listed["total"], 2);
    assert_eq!(listed["todos"][0]["id"], http["id"]);
    assert!(
        listed["todos"][1].get("body").is_none(),
        "list stays compact"
    );
    let paged =
        call(&app, &session, "todo_list", json!({"limit":1})).await["structuredContent"].clone();
    assert_eq!(paged["todos"].as_array().unwrap().len(), 1);
    assert_eq!(paged["next_offset"], 1);

    let updated = call(
        &app,
        &session,
        "todo_update",
        json!({"id":id,"due":"2026-02-01","url":null}),
    )
    .await;
    assert_eq!(updated["structuredContent"]["due"], "2026-02-01");
    assert!(updated["structuredContent"]["url"].is_null());
    let completed = call(&app, &session, "todo_complete", json!({"id":id})).await;
    assert!(completed["structuredContent"]["done_at"].is_string());
    assert_eq!(
        call(&app, &session, "todo_list", json!({})).await["structuredContent"]["total"],
        1
    );
    assert_eq!(
        call(&app, &session, "todo_list", json!({"done":true})).await["structuredContent"]["todos"]
            [0]["id"],
        id
    );
    let got = call(&app, &session, "todo_get", json!({"id":id})).await;
    assert_eq!(got["structuredContent"]["body"], "memo");
}

#[tokio::test]
async fn mcp_reports_invalid_arguments_as_tool_errors() {
    let (_dir, app) = server();
    let session = session(&app).await;
    let rejected = call(
        &app,
        &session,
        "todo_add",
        json!({"title":"x","url":"javascript:alert(1)"}),
    )
    .await;
    assert_eq!(rejected["isError"], true);
    assert_eq!(rejected["structuredContent"]["code"], "invalid");
    let missing = call(&app, &session, "todo_get", json!({"id":"missing"})).await;
    assert_eq!(missing["structuredContent"]["code"], "not_found");
}
