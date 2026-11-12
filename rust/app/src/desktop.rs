// Masaustu entegrasyonu: tepsi, global kisayol, bildirim, tek ornek, dosya izleme,
// acilista baslat, klasor secme. Arayuz bunlari sadece Event ve asagidaki
// fonksiyonlar uzerinden gorur.
// SOZLESME (patron yazdi): imzalari degistirme; degismesi gerekirse patrona yaz.
use crate::config::Config;
use notix_core::Store;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// tepsi tiklamasi / "Goster" / ikinci acilis denemesi
    ShowWindow,
    /// bildirime tiklandi: pencereyi goster ve Bugun ekranini ac
    ShowToday,
    /// global hizli ekleme kisayolu ya da tepsi menusu
    QuickAdd,
    /// tepsi menusu "Cik"
    Quit,
    /// not klasorunde <slug>.md degisti (Notix'in kendisi ya da disaridan AI/editor)
    FileChanged(String),
}

pub struct Desktop {
    /// Olay gelince ctx.request_repaint() ile arayuz uyandirilir; arayuz logic()'te bosaltir.
    pub rx: Receiver<Event>,
}

impl Desktop {
    /// main'de, eframe baslamadan once: false = baska ornek calisiyor (ona ShowWindow iletildi), cik.
    pub fn single_instance() -> bool {
        true
    }

    /// eframe creator icinde cagrilir (tepsi ve kisayol olay dongusu orada hazir).
    pub fn start(ctx: eframe::egui::Context, cfg: &Config, notes: &Path) -> Desktop {
        let _ = (ctx, cfg, notes);
        let (_tx, rx) = std::sync::mpsc::channel();
        Desktop { rx }
    }

    /// Ayar degisince: kisayolu yeniden bagla, tepsi menusunu dile gore kur, acilista baslat.
    /// Donus: hizli ekleme kisayolu baglanabildi mi (false = baska uygulamada).
    pub fn apply(&mut self, cfg: &Config) -> bool {
        let _ = cfg;
        true
    }

    /// Saatte bir cagrilir; notify acik ve bugun henuz bildirilmediyse tarihi gelen isler icin bildirim.
    pub fn notify_due(&mut self, store: &Store, cfg: &Config) {
        let _ = (store, cfg);
    }

    /// Sistem klasor secme penceresi (iptal = None).
    pub fn pick_folder() -> Option<PathBuf> {
        None
    }

    /// Klasoru isletim sisteminin dosya gezgininde ac.
    pub fn open_folder(dir: &Path) {
        let _ = dir;
    }

    /// Baglantiyi varsayilan tarayicida ac (sadece http/https).
    pub fn open_url(url: &str) {
        let _ = url;
    }
}
