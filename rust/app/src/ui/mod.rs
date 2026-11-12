// Arayuz. SOZLESME (patron yazdi): App::new imzasi main.rs'le bagli.
use crate::config::Config;
use crate::desktop::{Desktop, Event};
use eframe::egui;
use notix_core::Store;

pub struct App {
    store: Store,
    cfg: Config,
    desktop: Desktop,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, store: Store, cfg: Config) -> App {
        let desktop = Desktop::start(cc.egui_ctx.clone(), &cfg, store.dir());
        App { store, cfg, desktop }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        while let Ok(ev) = self.desktop.rx.try_recv() {
            if ev == Event::Quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _f: &mut eframe::Frame) {
        ui.label(format!("Notix · {} · {}", self.store.dir().display(), self.cfg.theme));
    }
}
