// Esdegerlik: ayni girdi dosyalarinda JS store.js ile Rust store ayni cikti dosyalarini /
// ayni JSON'u uretmeli. node'u child process olarak cagirir; node yoksa atlanir.
mod common;
use common::{store, tmp};
use notix_core::{Dir, Kind, Store, TaskUpdate};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MESSY: &str = "# Mess\n\nserbest metin\n\t- [ ] tab is @2026-01-01\n- [x] bitti\n  - [ ] alt  \n####### yedi diyez\n## Notlar\nelle yazilmis\n- [ ] ikinci @2099-05-05 x\n\n## Bellek\n\nhafiza satiri\n";
const NO_MEM: &str = "# Duz\n- [ ] a\n\n\n";

fn node_ok() -> bool {
    let ok = Command::new("node").arg("-v").output().is_ok();
    if !ok {
        eprintln!("node yok, esdegerlik testi atlandi");
    }
    ok
}

fn store_js() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../store.js")
}

fn js(dir: &Path, code: &str) -> String {
    let src = format!("const s=require({:?});const out=(v)=>console.log(JSON.stringify(v));{code}", store_js());
    let o = Command::new("node")
        .args(["-e", &src])
        .env("NOTIX_DIR", dir)
        .env("NOTIX_BACKUP_MS", "3600000")
        .output()
        .unwrap();
    assert!(o.status.success(), "node: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn seed(dir: &Path, files: &[(&str, &str)]) {
    for (n, c) in files {
        fs::write(dir.join(format!("{n}.md")), c).unwrap();
    }
}

fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().into(), fs::read_to_string(e.path()).unwrap()))
        .collect()
}

// Ayni dosyalar, Rust kodu bir dizinde, JS kodu oburunde; sonuc dosyalari ayni olmali.
fn same_files(files: &[(&str, &str)], rust: impl Fn(&Store), js_code: &str) {
    if !node_ok() {
        return;
    }
    let (a, b) = (store(), tmp());
    seed(a.dir(), files);
    seed(&b, files);
    rust(&a);
    js(&b, js_code);
    assert_eq!(snapshot(a.dir()), snapshot(&b), "js: {js_code}");
}

// Ayni dosyalar; salt okunur sonuc JSON'u ayni olmali.
fn same_json(files: &[(&str, &str)], rust: impl Fn(&Store) -> Value, js_expr: &str) {
    if !node_ok() {
        return;
    }
    let (a, b) = (store(), tmp());
    seed(a.dir(), files);
    seed(&b, files);
    let got: Value = serde_json::from_str(&js(&b, &format!("out({js_expr})"))).unwrap();
    assert_eq!(rust(&a), got, "js: {js_expr}");
}

fn upd<'a>(expect: Option<&'a str>, done: Option<bool>, text: Option<&'a str>, remove: bool) -> TaskUpdate<'a> {
    TaskUpdate { expect, done, text, remove }
}

#[test]
fn add_task() {
    for f in [MESSY, NO_MEM, "", "# X\r\n- [ ] a\r\n## Memory\r\nm\r\n"] {
        same_files(&[("p", f)], |s| s.add_task("p", "yeni\n  - [ ] alt\n\n  not  \n\n", Kind::Task).unwrap(), "s.addTask('p','yeni\\n  - [ ] alt\\n\\n  not  \\n\\n')");
        same_files(&[("p", f)], |s| s.add_task("p", "  \n\talt is  ", Kind::Sub).unwrap(), "s.addTask('p','  \\n\\talt is  ','sub')");
        same_files(&[("p", f)], |s| s.add_task("p", "  serbest  ", Kind::Note).unwrap(), "s.addTask('p','  serbest  ','note')");
    }
}

#[test]
fn update_task() {
    same_files(&[("p", MESSY)], |s| s.update_task("p", 3, upd(Some("tab is @2026-01-01"), Some(true), None, false)).unwrap(), "s.updateTask('p',3,{expect:'tab is @2026-01-01',done:true})");
    same_files(&[("p", MESSY)], |s| s.update_task("p", 4, upd(None, Some(false), Some("  yeni\r\n\n metin  "), false)).unwrap(), "s.updateTask('p',4,{done:false,text:'  yeni\\r\\n\\n metin  '})");
    same_files(&[("p", MESSY)], |s| s.update_task("p", 5, upd(None, None, Some("x"), false)).unwrap(), "s.updateTask('p',5,{text:'x'})");
    same_files(&[("p", MESSY)], |s| s.update_task("p", 3, upd(None, None, None, true)).unwrap(), "s.updateTask('p',3,{remove:true})");
    same_files(&[("p", MESSY)], |s| assert_eq!(s.update_task("p", 2, upd(None, Some(true), None, false)).unwrap_err().status(), 409), "try{s.updateTask('p',2,{done:true})}catch(e){if(e.code!==409)throw e}");
}

#[test]
fn set_memory() {
    for f in [MESSY, NO_MEM, "# Bos\n", ""] {
        same_files(&[("p", f)], |s| s.set_memory("p", "\n\nilk\r\nikinci\n\n", None, Some("Memory")).unwrap(), "s.setMemory('p','\\n\\nilk\\r\\nikinci\\n\\n',null,'Memory')");
        same_files(&[("p", f)], |s| s.set_memory("p", "yeni", None, None).unwrap(), "s.setMemory('p','yeni')");
        same_files(&[("p", f)], |s| s.set_memory("p", "", None, None).unwrap(), "s.setMemory('p','')");
    }
    same_files(&[("p", MESSY)], |s| s.set_memory("p", "degisti", Some("hafiza satiri"), None).unwrap(), "s.setMemory('p','degisti','hafiza satiri')");
}

#[test]
fn move_task() {
    for (i, dir, js_dir) in [(3, Dir::Down, "down"), (3, Dir::Up, "up"), (4, Dir::Up, "up"), (9, Dir::Down, "down"), (9, Dir::Up, "up")] {
        same_files(&[("p", MESSY)], |s| s.move_task("p", i, dir, None).unwrap(), &format!("s.moveTask('p',{i},'{js_dir}')"));
    }
}

#[test]
fn rename_ve_tasima() {
    same_files(&[("eski", MESSY)], |s| assert_eq!(s.rename("eski", "  Yeni\r\nİş  Adı ").unwrap(), "yeni-is-adi"), "s.rename('eski','  Yeni\\r\\nİş  Adı ')");
    same_files(&[("eski", "- [ ] basliksiz\n")], |s| { s.rename("eski", "Baslik").unwrap(); }, "s.rename('eski','Baslik')");
    same_files(&[("a", MESSY), ("b", NO_MEM)], |s| { s.move_to_project("a", 3, "b", None).unwrap(); }, "s.moveToProject('a',3,'b')");
    same_files(&[("a", MESSY), ("b", MESSY)], |s| { s.move_to_project("a", 9, "b", Some("ikinci @2099-05-05 x")).unwrap(); }, "s.moveToProject('a',9,'b','ikinci @2099-05-05 x')");
}

#[test]
fn import_export_ve_belge_modu() {
    same_files(&[("p", MESSY)], |s| { s.import_file("p", "# Yeni\r\nx\r\n").unwrap(); s.write("p", "a\r\nb", Some("# Yeni\nx\n")).unwrap(); }, "s.importFile('p','# Yeni\\r\\nx\\r\\n');s.write('p','a\\r\\nb','# Yeni\\nx\\n')");
}

#[test]
fn okuma_sonuclari_ayni_json() {
    let files = [("mess", MESSY), ("crlf", "# Crlf\r\n- [ ] a @2020-02-02\r\n\u{a0}- [x] nbsp\r\nser\u{2028}best\r\n"), ("duz", NO_MEM)];
    same_json(&files, |s| s.parse("mess").unwrap().json(), "s.parse('mess')");
    same_json(&files, |s| s.parse("crlf").unwrap().json(), "s.parse('crlf')");
    same_json(&files, |s| Value::Array(s.list().unwrap().iter().map(|x| x.json()).collect()), "s.list()");
    same_json(&files, |s| Value::Array(s.search("A").unwrap().iter().map(|x| x.json()).collect()), "s.search('A')");
    same_json(&files, |s| Value::Array(s.search(" SERBEST ").unwrap().iter().map(|x| x.json()).collect()), "s.search(' SERBEST ')");
    same_json(&files, |s| Value::Array(s.due("2030-01-01").unwrap().iter().map(|x| x.json()).collect()), "s.due('2030-01-01')");
    same_json(&files, |s| { let m = s.memory("mess").unwrap(); json!({"text": m.text, "from": m.from, "to": m.to}) }, "s.memory('mess')");
    same_json(&files, |s| json!(s.export_all().unwrap()), "s.exportAll()");
}
