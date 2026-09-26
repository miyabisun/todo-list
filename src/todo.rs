//! A to-do is one Markdown file: YAML frontmatter for the fields, the memo as body.
use crate::Error;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub done_at: Option<String>,
    /// Hand-added frontmatter fields survive server edits.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Todo {
    pub id: String,
    #[serde(flatten)]
    pub meta: Meta,
    pub body: String,
}

/// IDs name files, so only path-safe ASCII is accepted. New IDs are UUIDs.
pub fn check_id(id: &str) -> Result<(), Error> {
    let safe = (1..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if safe {
        Ok(())
    } else {
        Err(Error::NotFound(format!("todo not found: {id}")))
    }
}

pub fn parse_document(id: &str, text: &str) -> Result<Todo, Error> {
    let invalid = |message: &str| Error::Invalid(format!("{id}.md: {message}"));
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| invalid("missing opening ---"))?;
    let (yaml, body) = if let Some(body) = rest.strip_prefix("---\n") {
        ("", body)
    } else if let Some(index) = rest.find("\n---\n") {
        (&rest[..=index], &rest[index + 5..])
    } else if let Some(yaml) = rest.strip_suffix("\n---") {
        (yaml, "")
    } else {
        return Err(invalid("missing closing ---"));
    };
    let meta: Meta = serde_norway::from_str(yaml).map_err(|e| invalid(&e.to_string()))?;
    if meta.extra.contains_key("id") || meta.extra.contains_key("body") {
        return Err(invalid(
            "id comes from the file name and the memo follows the closing ---",
        ));
    }
    Ok(Todo {
        id: id.into(),
        meta,
        body: body.into(),
    })
}

pub fn render_document(todo: &Todo) -> Result<String, Error> {
    let yaml =
        serde_norway::to_string(&todo.meta).map_err(|e| Error::Invalid(format!("yaml: {e}")))?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    Ok(format!("---\n{yaml}---\n{}", todo.body))
}

fn one_line(key: &str, value: &Value) -> Result<String, Error> {
    let text = value.as_str().map(str::trim).unwrap_or_default();
    if text.is_empty() || text.contains(['\n', '\r']) {
        return Err(Error::Invalid(format!("{key} must be one nonblank line")));
    }
    Ok(text.into())
}

fn optional(
    key: &str,
    value: &Value,
    check: impl Fn(&str) -> bool,
    hint: &str,
) -> Result<Option<String>, Error> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) if check(text) => Ok(Some(text.clone())),
        _ => Err(Error::Invalid(format!("{key} must be {hint} or null"))),
    }
}

/// Only absolute http(s) links are stored, so a tap never runs script.
fn is_web_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    rest.is_some_and(|host| !host.is_empty() && !host.starts_with('/'))
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn is_date(text: &str) -> bool {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    text.len() == 10 && time::Date::parse(text, format).is_ok()
}

/// Apply validated fields to a to-do. `now` stamps `updated_at` and completion.
/// Nothing changes unless every field is valid.
pub fn apply(todo: &mut Todo, fields: &Value, now: &str) -> Result<(), Error> {
    let fields = fields
        .as_object()
        .ok_or_else(|| Error::Invalid("fields must be a JSON object".into()))?;
    let mut next = todo.clone();
    for (key, value) in fields {
        match key.as_str() {
            "title" => next.meta.title = one_line(key, value)?,
            "source" => {
                next.meta.source = match value {
                    Value::Null => None,
                    _ => Some(one_line(key, value)?),
                };
            }
            "url" => next.meta.url = optional(key, value, is_web_url, "an http(s) URL")?,
            "due" => next.meta.due = optional(key, value, is_date, "a YYYY-MM-DD date")?,
            "body" => {
                next.body = value
                    .as_str()
                    .ok_or_else(|| Error::Invalid("body must be a string".into()))?
                    .into();
            }
            "done" => match value.as_bool() {
                Some(true) => {
                    next.meta.done_at.get_or_insert_with(|| now.into());
                }
                Some(false) => next.meta.done_at = None,
                None => return Err(Error::Invalid("done must be true or false".into())),
            },
            _ => {
                return Err(Error::Invalid(format!("unknown or read-only field: {key}")));
            }
        }
    }
    next.meta.updated_at = now.into();
    *todo = next;
    Ok(())
}

/// Open items first by due date (undated last) then age; done items newest first.
pub fn sort(todos: &mut [Todo]) {
    todos.sort_by(|a, b| match (&a.meta.done_at, &b.meta.done_at) {
        (None, None) => {
            let key = |t: &Todo| {
                (
                    t.meta.due.is_none(),
                    t.meta.due.clone(),
                    t.meta.created_at.clone(),
                    t.id.clone(),
                )
            };
            key(a).cmp(&key(b))
        }
        (Some(x), Some(y)) => y.cmp(x).then_with(|| a.id.cmp(&b.id)),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn blank(id: &str) -> Todo {
        Todo {
            id: id.into(),
            meta: Meta {
                title: "placeholder".into(),
                url: None,
                due: None,
                source: None,
                created_at: "2026-01-01T00:00:00Z".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
                done_at: None,
                extra: Map::new(),
            },
            body: String::new(),
        }
    }

    fn applied(fields: &Value) -> Result<Todo, Error> {
        let mut todo = blank("a");
        apply(&mut todo, fields, "2026-02-02T00:00:00Z")?;
        Ok(todo)
    }

    #[test]
    fn ids_are_path_safe_names() {
        for id in ["3f2c1a9e-0000-4000-8000-000000000000", "buy_milk-2"] {
            assert!(check_id(id).is_ok(), "{id}");
        }
        let long = "a".repeat(129);
        for id in [
            "",
            "../x",
            "a/b",
            "a.md",
            ".hidden",
            "日本語",
            " a",
            long.as_str(),
        ] {
            assert!(check_id(id).is_err(), "{id:?}");
        }
    }

    #[test]
    fn documents_round_trip_with_verbatim_body_and_unknown_fields() {
        let text = "---\ntitle: 牛乳を買う\nurl: https://example.com/list?a=1&b=2\ndue: 2026-10-01\nsource: example-agent\ncreated_at: 2026-09-01T00:00:00Z\nupdated_at: 2026-09-02T00:00:00Z\ndone_at: null\ntags:\n- errand\n---\n# メモ\n\n---\nsecond part\n";
        let todo = parse_document("milk", text).unwrap();
        assert_eq!(todo.id, "milk");
        assert_eq!(todo.meta.title, "牛乳を買う");
        assert_eq!(todo.meta.due.as_deref(), Some("2026-10-01"));
        assert_eq!(todo.meta.extra["tags"], json!(["errand"]));
        assert_eq!(todo.body, "# メモ\n\n---\nsecond part\n");
        let again = parse_document("milk", &render_document(&todo).unwrap()).unwrap();
        assert_eq!(again, todo);
    }

    #[test]
    fn documents_without_a_body_or_optional_fields_parse() {
        let todo = parse_document("a", "---\ntitle: t\ncreated_at: c\nupdated_at: u\n---").unwrap();
        assert_eq!(todo.body, "");
        assert_eq!(todo.meta.url, None);
        assert_eq!(todo.meta.done_at, None);
    }

    #[test]
    fn malformed_documents_are_rejected() {
        for text in [
            "title: t\n",
            "---\ntitle: t\ncreated_at: c\nupdated_at: u\n",
            "---\n- a list\n---\n",
            "---\ncreated_at: c\nupdated_at: u\n---\n",
            // The file name is the id and the memo follows the fence; copies would shadow them.
            "---\ntitle: t\ncreated_at: c\nupdated_at: u\nid: other\n---\n",
            "---\ntitle: t\ncreated_at: c\nupdated_at: u\nbody: hidden\n---\n",
        ] {
            assert!(parse_document("a", text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn apply_sets_fields_and_stamps_the_update() {
        let todo = applied(&json!({
            "title": "  書類を出す  ",
            "url": "HTTPS://example.com/form",
            "due": "2026-02-28",
            "source": "example-app",
            "body": "持ち物: 印鑑"
        }))
        .unwrap();
        assert_eq!(todo.meta.title, "書類を出す");
        assert_eq!(todo.meta.url.as_deref(), Some("HTTPS://example.com/form"));
        assert_eq!(todo.meta.due.as_deref(), Some("2026-02-28"));
        assert_eq!(todo.meta.source.as_deref(), Some("example-app"));
        assert_eq!(todo.body, "持ち物: 印鑑");
        assert_eq!(todo.meta.updated_at, "2026-02-02T00:00:00Z");
        assert_eq!(todo.meta.created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn null_clears_optional_fields() {
        let mut todo =
            applied(&json!({"url":"http://example.com","due":"2026-03-01","source":"x"})).unwrap();
        apply(
            &mut todo,
            &json!({"url":null,"due":null,"source":null}),
            "later",
        )
        .unwrap();
        assert_eq!(
            (todo.meta.url, todo.meta.due, todo.meta.source),
            (None, None, None)
        );
    }

    #[test]
    fn completion_keeps_the_first_time_and_reopening_clears_it() {
        let mut todo = applied(&json!({"done": true})).unwrap();
        assert_eq!(todo.meta.done_at.as_deref(), Some("2026-02-02T00:00:00Z"));
        apply(&mut todo, &json!({"done": true}), "2026-03-03T00:00:00Z").unwrap();
        assert_eq!(todo.meta.done_at.as_deref(), Some("2026-02-02T00:00:00Z"));
        apply(&mut todo, &json!({"done": false}), "2026-03-04T00:00:00Z").unwrap();
        assert_eq!(todo.meta.done_at, None);
    }

    #[test]
    fn invalid_fields_change_nothing() {
        for fields in [
            json!([]),
            json!({"title": "   "}),
            json!({"title": "two\nlines"}),
            json!({"title": null}),
            json!({"url": "javascript:alert(1)"}),
            json!({"url": "https://"}),
            json!({"url": "https://exa mple.com"}),
            json!({"url": "ftp://example.com"}),
            json!({"due": "2026-02-30"}),
            json!({"due": "2026/02/01"}),
            json!({"due": "2026-2-1"}),
            json!({"source": "  "}),
            json!({"source": "a\nb"}),
            json!({"body": null}),
            json!({"done": "yes"}),
            json!({"id": "other"}),
            json!({"created_at": "x"}),
            json!({"title": "ok", "unknown": 1}),
        ] {
            let mut todo = blank("a");
            let before = todo.clone();
            assert!(apply(&mut todo, &fields, "now").is_err(), "{fields}");
            assert_eq!(todo, before, "{fields}");
        }
    }

    #[test]
    fn sort_puts_open_items_by_due_then_age_and_done_items_newest_first() {
        let item = |id: &str, due: Option<&str>, created: &str, done: Option<&str>| {
            let mut todo = blank(id);
            todo.meta.due = due.map(Into::into);
            todo.meta.created_at = created.into();
            todo.meta.done_at = done.map(Into::into);
            todo
        };
        let mut todos = vec![
            item("undated-old", None, "2026-01-01", None),
            item(
                "done-old",
                Some("2026-01-01"),
                "2026-01-01",
                Some("2026-01-05"),
            ),
            item("due-late", Some("2026-05-01"), "2026-01-01", None),
            item("undated-new", None, "2026-01-03", None),
            item("done-new", None, "2026-01-01", Some("2026-01-09")),
            item("due-soon-b", Some("2026-04-01"), "2026-01-02", None),
            item("due-soon-a", Some("2026-04-01"), "2026-01-02", None),
        ];
        sort(&mut todos);
        let ids: Vec<_> = todos.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "due-soon-a",
                "due-soon-b",
                "due-late",
                "undated-old",
                "undated-new",
                "done-new",
                "done-old"
            ]
        );
    }
}
