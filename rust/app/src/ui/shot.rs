// `notix --shot <klasor>`: ornek not klasorunde (NOTIX_DIR) ekranlarin PNG'sini kaydedip cikar.
// Gorsel dogrulama icin; normal kullanimda hic calismaz.
use super::project::{Edit, MemEdit};
use super::{quick, App, Confirm, Screen};
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type Setup = fn(&mut App);

pub struct Driver {
    dir: PathBuf,
    step: usize,
    wait: u32,
    pending: Option<Arc<AtomicBool>>,
    t0: std::time::Instant,
}

fn look(a: &mut App, lang: &str, theme: &str, accent: &str) {
    a.cfg.lang = lang.into();
    a.cfg.theme = theme.into();
    a.cfg.accent = accent.into();
    (a.doc, a.guide, a.confirm, a.edit, a.mem_edit) = (None, None, None, None, None);
    a.settings.open = false;
    a.hit = None;
    a.find.clear();
    a.hot.clear();
}

fn proj(a: &mut App) {
    a.open("notix");
}

const STEPS: &[(&str, Setup)] = &[
    ("proj-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a) }),
    ("proj-light-en", |a| { look(a, "en", "light", "patina"); proj(a) }),
    ("proj-paper-en", |a| { look(a, "en", "paper", "patina"); proj(a) }),
    ("proj-forest-blue-en", |a| { look(a, "en", "forest", "blue"); proj(a) }),
    ("proj-dark-ja", |a| { look(a, "ja", "dark", "patina"); proj(a) }),
    ("proj-dark-ru", |a| { look(a, "ru", "dark", "patina"); proj(a) }),
    ("proj-dark-zh", |a| { look(a, "zh", "dark", "patina"); proj(a) }),
    ("proj-dark-tr-hit", |a| { look(a, "tr", "dark", "patina"); a.go_hit("notix", "Bellek panelini yaz @2020-01-01"); }),
    ("today-dark-en", |a| { look(a, "en", "dark", "patina"); a.set_screen(Screen::Today) }),
    ("trash-dark-en", |a| { look(a, "en", "dark", "patina"); a.set_screen(Screen::Trash) }),
    ("settings-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.settings.open = true }),
    ("doc-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.open_doc("notix") }),
    ("search-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.find = "task".into() }),
    ("edit-dark-en", |a| {
        look(a, "en", "dark", "patina");
        proj(a);
        let t = "Bellek panelini yaz @2020-01-01".to_string();
        a.edit = Some(Edit { i: 6, expect: t.clone(), text: t, fresh: true });
    }),
    ("memedit-dark-en", |a| {
        look(a, "en", "dark", "patina");
        proj(a);
        let t = a.loaded.as_ref().map(|l| l.mem.text.clone()).unwrap_or_default();
        a.mem_edit = Some(MemEdit { expect: t.clone(), text: t });
    }),
    ("confirm-dark-en", |a| {
        look(a, "en", "dark", "patina");
        proj(a);
        a.confirm = Some(Confirm::DelLine { i: 6, text: "Bellek panelini yaz @2020-01-01".into() });
    }),
    ("changed-dark-en", |a| {
        look(a, "en", "dark", "patina");
        proj(a);
        a.hot.insert("Fix `store.js` conflict handling @2026-11-12".into(), std::time::Instant::now());
        a.hot.insert("future task @2099-12-31".into(), std::time::Instant::now());
        a.say("File changed · 2 lines".into());
    }),
    ("guide1-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.guide = Some(0) }),
    ("guide4-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.guide = Some(3) }),
    ("quick-dark-en", |a| { look(a, "en", "dark", "patina"); proj(a); a.quick_shot_pending = true }),
];

impl Driver {
    pub fn from_args() -> Option<Driver> {
        let args: Vec<String> = std::env::args().collect();
        let i = args.iter().position(|a| a == "--shot")?;
        let dir = PathBuf::from(args.get(i + 1)?);
        let _ = std::fs::create_dir_all(&dir);
        Some(Driver { dir, step: 0, wait: 0, pending: None, t0: std::time::Instant::now() })
    }
}

// resimler arka planda cozuldugu icin kare degil sure bekler
const WAIT_MS: u128 = 1200;

/// ui()'nin sonunda her karede: sirayla kurar, bekler, goruntu ister, sonunda kapatir.
pub fn step(app: &mut App, ctx: &egui::Context) {
    let Some(mut d) = app.shot.take() else { return };
    ctx.request_repaint();
    if let Some(flag) = &d.pending {
        if !flag.load(Ordering::SeqCst) {
            app.shot = Some(d);
            return;
        }
        d.pending = None;
        d.step += 1;
        d.wait = 0;
    }
    let Some((name, setup)) = STEPS.get(d.step) else {
        app.quit(ctx);
        app.shot = Some(d);
        return;
    };
    if d.wait == 0 {
        d.t0 = std::time::Instant::now();
        setup(app);
        if app.quick_shot_pending {
            app.quick_shot_pending = false;
            let flag = Arc::new(AtomicBool::new(false));
            quick::open(app, ctx);
            app.quick.lock().unwrap().shot = Some((d.dir.join(format!("{name}.png")), flag.clone()));
            d.pending = Some(flag);
            app.shot = Some(d);
            return;
        }
    }
    d.wait += 1;
    if d.wait > 3 && d.t0.elapsed().as_millis() > WAIT_MS && d.pending.is_none() {
        let flag = Arc::new(AtomicBool::new(false));
        let (path, f) = (d.dir.join(format!("{name}.png")), flag.clone());
        ctx.request_screenshot(move |img| {
            let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            let r = image::save_buffer(&path, &raw, img.size[0] as u32, img.size[1] as u32, image::ColorType::Rgba8);
            println!("shot {} {}x{} ok={}", path.display(), img.size[0], img.size[1], r.is_ok());
            f.store(true, Ordering::SeqCst);
        });
        d.pending = Some(flag);
    }
    app.shot = Some(d);
}
