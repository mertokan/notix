mod mcp;

use notix_core::Store;
use std::path::PathBuf;

// NOTIX_DIR, yoksa Belgeler/Notix — kurulu Electron surumuyle ayni klasor,
// ikisi ayni notlari acabilsin.
fn notes_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("NOTIX_DIR").filter(|d| !d.is_empty()) {
        return d.into();
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).unwrap_or_default();
    PathBuf::from(home).join("Documents").join("Notix")
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--mcp") => mcp::run(&Store::new(notes_dir())),
        _ => {
            eprintln!("Notix {}\nKullanim: notix --mcp   (MCP sunucusu, stdio; not klasoru: NOTIX_DIR)", env!("CARGO_PKG_VERSION"));
            std::process::exit(2);
        }
    }
}
