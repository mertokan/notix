// mcp.js ile Rust --mcp ayni protokol davranisini gostermeli.
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_notix");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn node_ok() -> bool {
    Command::new("node").arg("-v").output().is_ok()
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("notix-mcp-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// Girdi satirlarini verip stdin'i kapatir, stdout satirlarini JSON olarak dondurur.
fn session(mut cmd: Command, dir: &Path, input: &[&str]) -> Vec<Value> {
    let mut p = cmd
        .env("NOTIX_DIR", dir)
        .env("NOTIX_BACKUP_MS", "3600000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut stdin = p.stdin.take().unwrap();
    for l in input {
        writeln!(stdin, "{l}").unwrap();
    }
    drop(stdin);
    let out = p.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn rust() -> Command {
    let mut c = Command::new(BIN);
    c.arg("--mcp");
    c
}

fn js() -> Command {
    let mut c = Command::new("node");
    c.arg(root().join("mcp.js"));
    c
}

#[test]
fn mcp_test_js_rust_binarye_karsi_gecer() {
    if !node_ok() {
        return;
    }
    let o = Command::new("node").arg(root().join("rust/app/tests/mcp-js-wrapper.js")).env("NOTIX_RUST_BIN", BIN).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

// Ayni girdi dizisi iki sunucuya verilir; cevaplar (zamana bagli olmayanlar) birebir ayni olmali.
#[test]
fn cevaplar_jsle_birebir_ayni() {
    if !node_ok() {
        return;
    }
    let seed = |d: &Path| std::fs::write(d.join("ev.md"), "# Ev\n- [ ] fatura @2000-01-01\n  - [x] alt\nserbest\n## Bellek\n\nkira\n").unwrap();
    let call = |id: u32, name: &str, args: Value| json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}}).to_string();
    let input: Vec<String> = vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}}).to_string(),
        json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"protocolVersion": "1999-01-01"}}).to_string(),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string(),
        json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}).to_string(),
        json!({"jsonrpc": "2.0", "id": "x", "method": "tools/list"}).to_string(),
        json!({"jsonrpc": "2.0", "id": 4, "method": "nope/x"}).to_string(),
        json!({"jsonrpc": "2.0", "method": "nope/y"}).to_string(),
        "{kotu json".into(),
        "[1]".into(),
        "null".into(),
        json!({"jsonrpc": "2.0", "id": null, "method": "ping"}).to_string(),
        call(5, "list_projects", json!({})),
        call(6, "read_project", json!({"slug": "ev"})),
        call(7, "read_project", json!({"slug": "yok"})),
        call(8, "read_project", json!({"slug": 5})),
        call(9, "read_project", json!({"slug": "../x"})),
        call(10, "add_task", json!({"slug": "yeni", "text": "a", "create": true, "kind": "sub"})),
        call(11, "add_task", json!({"slug": "yok2", "text": "a"})),
        call(12, "complete_task", json!({"slug": "ev", "line": 1, "expect": "fatura @2000-01-01"})),
        call(13, "complete_task", json!({"slug": "ev", "line": 1, "expect": "eski"})),
        call(14, "complete_task", json!({"slug": "ev", "line": 1.5, "expect": "x"})),
        call(15, "complete_task", json!({"slug": "ev", "line": -1, "expect": "x"})),
        call(16, "reopen_task", json!({"slug": "ev", "line": 1, "expect": "fatura @2000-01-01"})),
        call(17, "update_task", json!({"slug": "ev", "line": 2, "expect": "alt", "text": "alt2"})),
        call(18, "due_tasks", json!({"date": "2001-01-01"})),
        call(19, "due_tasks", json!({"date": "bugun"})),
        call(20, "search", json!({"q": "FATURA"})),
        call(21, "search", json!({"q": "f"})),
        call(22, "read_memory", json!({"slug": "ev"})),
        call(23, "write_memory", json!({"slug": "ev", "text": "yeni kira", "expect": "kira"})),
        call(24, "write_memory", json!({"slug": "ev", "text": "x", "expect": "kira"})),
        call(25, "create_project", json!({"title": "Ev İşleri"})),
        call(26, "create_project", json!({})),
        call(27, "nope", json!({})),
        json!({"jsonrpc": "2.0", "id": 28, "method": "tools/call", "params": {}}).to_string(),
    ];
    let input: Vec<&str> = input.iter().map(String::as_str).collect();
    let (a, b) = (tmp("rs"), tmp("js"));
    seed(&a);
    seed(&b);
    let (r, j) = (session(rust(), &a, &input), session(js(), &b, &input));
    assert_eq!(r.len(), j.len());
    for (r, j) in r.iter().zip(&j) {
        assert_eq!(r, j);
    }
    for f in ["ev.md", "yeni.md", "ev-isleri.md"] {
        assert_eq!(std::fs::read(a.join(f)).unwrap(), std::fs::read(b.join(f)).unwrap(), "{f}");
    }
}
