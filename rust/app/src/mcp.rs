// Notix MCP sunucusu: stdio, satir ayrimli JSON-RPC 2.0 — mcp.js'in birebir karsiligi.
// Cekirdegi dogrudan kullanir. stdout SADECE protokol icin; log stderr'e.
use notix_core::{Code, Error, Item, Kind, Store, TaskUpdate};
use serde_json::{json, Map, Value};
use std::io::{BufRead, Write};

const PROTOCOLS: [&str; 4] = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];
const CHANGED: &str = "file changed, re-read the project and retry";
const NOT_FOUND: &str = "project not found (use list_projects, or add_task with create:true)";

fn str_prop(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({"name": name, "description": description, "inputSchema": {"type": "object", "properties": properties, "required": required, "additionalProperties": false}})
}

fn tools() -> Value {
    let slug = || str_prop("Project slug (file name without .md), e.g. \"home-chores\". Get it from list_projects.");
    let line = || json!({"type": "integer", "minimum": 0, "description": "Line index of the task, as shown by read_project."});
    let expect = || str_prop("Current text of the task (after \"- [ ] \"), exactly as read_project showed it. If the file changed since, the call fails: re-read.");
    json!([
        tool("list_projects", "List all projects with open and done task counts.", json!({}), &[]),
        tool("read_project", "Read a project: raw markdown file plus its tasks with line indexes (needed for complete/reopen/update). Read the \"## Bellek\" (memory) section first for context.", json!({"slug": slug()}), &["slug"]),
        tool("add_task", "Append a line to a project (inserted above the memory section). A due date goes inline as \"@YYYY-MM-DD\".", json!({
            "slug": slug(),
            "text": str_prop("Task text. Multi-line text is kept as-is; the first line gets the chosen kind."),
            "kind": {"type": "string", "enum": ["task", "sub", "note"], "description": "task (default), sub = indented subtask, note = plain text line."},
            "create": {"type": "boolean", "description": "Create the project if it does not exist."},
        }), &["slug", "text"]),
        tool("complete_task", "Mark a task done ([x]). Never delete finished tasks.", json!({"slug": slug(), "line": line(), "expect": expect()}), &["slug", "line", "expect"]),
        tool("reopen_task", "Mark a done task open again ([ ]).", json!({"slug": slug(), "line": line(), "expect": expect()}), &["slug", "line", "expect"]),
        tool("update_task", "Replace the text of a task, keeping its done state and indentation.", json!({"slug": slug(), "line": line(), "expect": expect(), "text": str_prop("New task text (single line).")}), &["slug", "line", "expect", "text"]),
        tool("due_tasks", "Open tasks across all projects whose @date is on or before the given day.", json!({"date": str_prop("YYYY-MM-DD; defaults to local today.")}), &[]),
        tool("search", "Search lines (tasks and notes) across all projects, case-insensitive, min 2 characters.", json!({"q": str_prop("Search text.")}), &["q"]),
        tool("read_memory", "Read the project memory (\"## Bellek\" section): decisions, context, open questions.", json!({"slug": slug()}), &["slug"]),
        tool("write_memory", "Replace the project memory section. Keep existing lines; add or update. Empty text removes the section.", json!({
            "slug": slug(),
            "text": str_prop("Full new memory text (markdown, without the heading)."),
            "expect": str_prop("Memory text exactly as read_memory returned it (\"\" if none)."),
        }), &["slug", "text", "expect"]),
        tool("create_project", "Create a new project (no-op if it exists). Returns its slug.", json!({"title": str_prop("Project title, e.g. \"Home Chores\".")}), &["title"]),
    ])
}

// Arac hatasi: istemciye giden metin (cekirdek hatalari mcp.js'teki gibi cevrilir).
struct ToolErr(String);

impl From<Error> for ToolErr {
    fn from(e: Error) -> Self {
        ToolErr(match e.code {
            Code::Conflict => CHANGED.into(),
            Code::Missing => NOT_FOUND.into(),
            _ => e.message,
        })
    }
}

type Res = Result<Value, ToolErr>;
type Args = Map<String, Value>;

fn need_str<'a>(a: &'a Args, k: &str) -> Result<&'a str, ToolErr> {
    a.get(k).and_then(Value::as_str).ok_or_else(|| ToolErr(format!("\"{k}\" must be a string")))
}

// JS: Number.isInteger — 3.0 da tamsayi. Negatif satir dosyada yok sayilir (409), o yuzden usize::MAX.
fn need_line(a: &Args, k: &str) -> Result<usize, ToolErr> {
    let n = a.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)));
    n.map(|n| usize::try_from(n).unwrap_or(usize::MAX)).ok_or_else(|| ToolErr(format!("\"{k}\" must be an integer")))
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn is_date(s: &str) -> bool {
    s.len() == 10 && s.bytes().enumerate().all(|(i, b)| if i == 4 || i == 7 { b == b'-' } else { b.is_ascii_digit() })
}

fn set_done(s: &Store, a: &Args, done: bool) -> Res {
    let (slug, line, expect) = (need_str(a, "slug")?, need_line(a, "line")?, need_str(a, "expect")?);
    s.update_task(slug, line, TaskUpdate { expect: Some(expect), done: Some(done), ..Default::default() })?;
    Ok("ok".into())
}

fn json_list<T>(items: &[T], f: impl Fn(&T) -> Value) -> Value {
    Value::Array(items.iter().map(f).collect())
}

fn call(s: &Store, name: &str, a: &Args) -> Res {
    match name {
        "list_projects" => Ok(json_list(&s.list()?, |x| x.json())),
        "read_project" => {
            let p = s.parse(need_str(a, "slug")?)?;
            let tasks: Vec<Value> = p
                .items
                .iter()
                .filter_map(|x| match x {
                    Item::Task { i, done, indent, text } => Some(json!({"line": i, "done": done, "indent": indent, "text": text})),
                    _ => None,
                })
                .collect();
            Ok(json!({"slug": p.slug, "title": p.title, "markdown": s.read(&p.slug)?, "tasks": tasks}))
        }
        "add_task" => {
            let mut slug = need_str(a, "slug")?.to_string();
            if truthy(a.get("create")) {
                slug = s.create(&slug)?;
            }
            s.add_task(&slug, need_str(a, "text")?, Kind::parse(a.get("kind").and_then(Value::as_str)))?;
            Ok(json!({"slug": slug, "ok": true}))
        }
        "complete_task" => set_done(s, a, true),
        "reopen_task" => set_done(s, a, false),
        "update_task" => {
            let (slug, line, expect, text) = (need_str(a, "slug")?, need_line(a, "line")?, need_str(a, "expect")?, need_str(a, "text")?);
            s.update_task(slug, line, TaskUpdate { expect: Some(expect), text: Some(text), ..Default::default() })?;
            Ok("ok".into())
        }
        "due_tasks" => {
            let d = match a.get("date") {
                None | Some(Value::Null) => notix_core::today(),
                Some(Value::String(d)) if is_date(d) => d.clone(),
                Some(_) => return Err(ToolErr("\"date\" must be YYYY-MM-DD".into())),
            };
            Ok(json_list(&s.due(&d)?, |x| x.json()))
        }
        "search" => Ok(json_list(&s.search(need_str(a, "q")?)?, |x| x.json())),
        "read_memory" => Ok(json!({"text": s.memory(need_str(a, "slug")?)?.text})),
        "write_memory" => {
            let (slug, text, expect) = (need_str(a, "slug")?, need_str(a, "text")?, need_str(a, "expect")?);
            s.set_memory(slug, text, Some(expect), None)?;
            Ok("ok".into())
        }
        "create_project" => Ok(json!({"slug": s.create(need_str(a, "title")?)?})),
        _ => unreachable!("known tools only"),
    }
}

const NAMES: [&str; 11] = [
    "list_projects", "read_project", "add_task", "complete_task", "reopen_task", "update_task",
    "due_tasks", "search", "read_memory", "write_memory", "create_project",
];

fn call_tool(s: &Store, params: &Value) -> Value {
    let name = params.get("name");
    let known = name.and_then(Value::as_str).filter(|n| NAMES.contains(n));
    let result = match known {
        None => Err(ToolErr(format!(
            "unknown tool: {}",
            match name {
                None => "undefined".into(),
                Some(Value::String(n)) => n.clone(),
                Some(v) => v.to_string(),
            }
        ))),
        Some(n) => {
            let empty = Map::new();
            let args = params.get("arguments").and_then(Value::as_object).unwrap_or(&empty);
            call(s, n, args)
        }
    };
    match result {
        Ok(v) => {
            let text = match v {
                Value::String(t) => t,
                v => serde_json::to_string_pretty(&v).unwrap_or_default(),
            };
            json!({"content": [{"type": "text", "text": text}]})
        }
        Err(ToolErr(text)) => json!({"content": [{"type": "text", "text": text}], "isError": true}),
    }
}

// Ok(None): bildirim, cevap yok. Err((kod, mesaj)): JSON-RPC hatasi.
fn handle(s: &Store, msg: &Map<String, Value>, method: &str) -> Result<Option<Value>, (i64, String)> {
    let params = msg.get("params").unwrap_or(&Value::Null);
    let result = match method {
        "initialize" => {
            let v = params.get("protocolVersion").and_then(Value::as_str).filter(|v| PROTOCOLS.contains(v)).unwrap_or("2025-06-18");
            json!({"protocolVersion": v, "capabilities": {"tools": {}}, "serverInfo": {"name": "notix", "version": env!("CARGO_PKG_VERSION")}})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools()}),
        "tools/call" => {
            if params.is_null() {
                return Err((-32603, "Cannot destructure 'undefined' or 'null'".into()));
            }
            call_tool(s, params)
        }
        _ if msg.contains_key("id") => return Err((-32601, format!("method not found: {method}"))),
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn send(out: &mut impl Write, m: Value) {
    let mut o = Map::new();
    o.insert("jsonrpc".into(), "2.0".into());
    if let Value::Object(m) = m {
        o.extend(m);
    }
    // stdout kapandiysa istemci gitmis demektir; yazamamak hata degil.
    let _ = writeln!(out, "{}", Value::Object(o));
    let _ = out.flush();
}

pub fn run(store: &Store) {
    let mut out = std::io::stdout().lock();
    let mut input = std::io::stdin().lock();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match input.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let raw = String::from_utf8_lossy(&buf);
        let raw = raw.trim_end_matches(['\n', '\r']);
        if raw.trim().is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
            send(&mut out, json!({"id": null, "error": {"code": -32700, "message": "parse error"}}));
            continue;
        };
        let Value::Object(msg) = parsed else {
            send(&mut out, json!({"id": null, "error": {"code": -32600, "message": "invalid request"}}));
            continue;
        };
        // method yoksa istemcinin cevabidir; biz istek gondermiyoruz.
        let Some(method) = msg.get("method").and_then(Value::as_str) else { continue };
        let id = msg.get("id").cloned();
        match (handle(store, &msg, method), id) {
            (Ok(Some(result)), Some(id)) => send(&mut out, json!({"id": id, "result": result})),
            (Err((code, message)), Some(id)) => send(&mut out, json!({"id": id, "error": {"code": code, "message": message}})),
            _ => {}
        }
    }
}
