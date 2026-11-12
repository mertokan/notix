// Windows'ta release derlemesi konsol penceresi acmasin. --mcp stdio'su yine calisir
// (istemci process'i pipe ile baslatir).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod desktop;
mod mcp;
mod ui;

use config::{notes_dir, Config};
use notix_core::Store;

fn main() -> eframe::Result {
    let cfg = Config::load();
    let store = Store::new(notes_dir(&cfg));
    if std::env::args().nth(1).as_deref() == Some("--mcp") {
        mcp::run(&store);
        return Ok(());
    }
    if !desktop::Desktop::single_instance() {
        return Ok(());
    }
    store.set_backup(cfg.backup);
    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Notix")
            .with_inner_size([1100.0, 780.0])
            .with_min_inner_size([420.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native("notix", opts, Box::new(move |cc| Ok(Box::new(ui::App::new(cc, store, cfg)))))
}
