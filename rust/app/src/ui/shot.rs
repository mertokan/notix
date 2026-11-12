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
    test: usize,
    tframe: u32,
    selftest: bool,
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
        let selftest = args.iter().any(|a| a == "--selftest");
        Some(Driver { dir, step: if selftest { STEPS.len() } else { 0 }, wait: 0, pending: None, t0: std::time::Instant::now(), test: 0, tframe: 0, selftest })
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
        if d.selftest && run_tests(app, &mut d) {
            app.shot = Some(d);
            return;
        }
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

// --- etkilesim testleri (--selftest): gercek olay akisi, sonuc dosyadan ve App durumundan okunur ---
struct Test {
    name: &'static str,
    setup: fn(&mut App),
    key: Option<(egui::Key, egui::Modifiers)>,
    check: fn(&mut App) -> bool,
}

fn line_of(a: &App, text: &str) -> usize {
    a.loaded.as_ref().and_then(|l| l.project.items.iter().find(|x| x.text().starts_with(text)).map(|x| x.i())).unwrap_or(0)
}
fn file(a: &App) -> String {
    a.store.read("notix").unwrap_or_default()
}

const TESTS: &[Test] = &[
    Test {
        name: "satir duzenle + Enter kaydeder",
        setup: |a| {
            look(a, "en", "dark", "patina");
            proj(a);
            let i = line_of(a, "future task");
            a.edit = Some(Edit { i, expect: "future task @2099-12-31".into(), text: "future edited @2099-12-31".into(), fresh: true });
        },
        key: Some((egui::Key::Enter, egui::Modifiers::NONE)),
        check: |a| a.edit.is_none() && file(a).contains("- [ ] future edited @2099-12-31"),
    },
    Test {
        name: "duzenle + Esc vazgecer",
        setup: |a| {
            let i = line_of(a, "future edited");
            a.edit = Some(Edit { i, expect: "future edited @2099-12-31".into(), text: "SHOULD NOT SAVE".into(), fresh: true });
        },
        key: Some((egui::Key::Escape, egui::Modifiers::NONE)),
        check: |a| a.edit.is_none() && !file(a).contains("SHOULD NOT SAVE"),
    },
    Test {
        name: "ekleme satirinda Enter gorev ekler (bellegin ustune)",
        setup: |a| {
            a.add.text = "added by selftest".into();
            a.focus_add_again();
        },
        key: Some((egui::Key::Enter, egui::Modifiers::NONE)),
        check: |a| {
            let f = file(a);
            a.add.text.is_empty() && f.find("- [ ] added by selftest").is_some_and(|p| f.find("## Memory").is_some_and(|m| p < m))
        },
    },
    Test {
        name: "expect uyusmazligi 409: dosya ezilmez, toast gelir",
        setup: |a| {
            let i = line_of(a, "added by selftest");
            let f = file(a).replace("added by selftest", "changed by AI");
            std::fs::write(a.store.dir().join("notix.md"), f).ok();
            a.update_task(i, "added by selftest", notix_core::TaskUpdate { done: Some(true), ..Default::default() });
        },
        key: None,
        check: |a| {
            let f = file(a);
            f.contains("- [ ] changed by AI") && !f.contains("[x] changed by AI") && a.toast_text().is_some_and(|t| t.contains(&a.t("changedRefreshed")))
        },
    },
    Test {
        name: "belge modu 409'da kapanmaz, yazilan kalir",
        setup: |a| {
            a.open_doc("notix");
            if let Some(d) = a.doc.as_mut() {
                d.text.push_str("
user typed text
");
            }
            let f = file(a) + "
external line
";
            std::fs::write(a.store.dir().join("notix.md"), f).ok();
            a.save_doc_for_test();
        },
        key: None,
        check: |a| a.doc.as_ref().is_some_and(|d| d.text.contains("user typed text")) && !file(a).contains("user typed text"),
    },
    Test {
        name: "bellek 409'da editor acik kalir",
        setup: |a| {
            a.doc = None;
            a.reload(false);
            let t = a.loaded.as_ref().map(|l| l.mem.text.clone()).unwrap_or_default();
            a.mem_edit = Some(MemEdit { expect: t.clone(), text: "my memory edit".into() });
            let f = file(a).replace("check this later", "AI changed memory");
            std::fs::write(a.store.dir().join("notix.md"), f).ok();
            a.save_mem_for_test();
        },
        key: None,
        check: |a| a.mem_edit.as_ref().is_some_and(|m| m.text == "my memory edit") && !file(a).contains("my memory edit"),
    },
    Test {
        name: "kisayol N ekleme kutusuna odaklar, harf yazilmaz",
        setup: |a| {
            a.mem_edit = None;
            a.reload(false);
        },
        key: Some((egui::Key::N, egui::Modifiers::NONE)),
        check: |a| a.add.text.is_empty(),
    },
];

/// true: testler suruyor. Her testte: kur → 8 kare bekle (odak oturur) → tus → 4 kare → kontrol.
fn run_tests(app: &mut App, d: &mut Driver) -> bool {
    let Some(t) = TESTS.get(d.test) else { return false };
    if d.tframe == 0 {
        (t.setup)(app);
    }
    d.tframe += 1;
    if d.tframe == 10 {
        if let Some(k) = t.key {
            app.inject = Some(k);
        }
    }
    if d.tframe == 16 {
        let ok = (t.check)(app);
        println!("TEST {} : {}", if ok { "PASS" } else { "FAIL" }, t.name);
        d.test += 1;
        d.tframe = 0;
    }
    true
}
