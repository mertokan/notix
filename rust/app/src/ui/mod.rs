// Arayuz. SOZLESME (patron yazdi): App::new imzasi main.rs'le bagli.
// Electron surumunun (public/app.js) ekran ekran karsiligi; mantik orada, gorunum style.css'te.
mod guide;
mod i18n;
mod md;
mod project;
mod quick;
mod screens;
mod settings;
mod shot;
mod theme;
mod w;

use crate::config::Config;
use crate::desktop::{Desktop, Event};
use eframe::egui::{self, vec2, Align, Align2, Key, Modifiers, Rect, Stroke};
use i18n::I18n;
use notix_core::{Due, Hit, Memory, Project, Store, Summary, TrashEntry};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::Pal;

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Welcome,
    Project,
    Today,
    Trash,
}

pub enum Confirm {
    DelLine { i: usize, text: String },
    TrashProject(String),
    Purge(TrashEntry),
    Import(BTreeMap<String, String>),
    DiscardDoc,
}

pub struct Loaded {
    pub project: Project,
    pub mem: Memory,
}

pub struct App {
    pub store: Arc<Store>,
    pub cfg: Config,
    desktop: Desktop,
    pub i18n: Arc<I18n>,
    fonts: theme::Fonts,
    pub pal: Pal,
    applied: (String, String, bool, String),
    pub quick: Arc<Mutex<quick::State>>,
    pub shot: Option<shot::Driver>,
    pub quick_shot_pending: bool,
    quitting: bool,
    visible: bool,

    pub screen: Screen,
    pub cur: Option<String>,
    pub loaded: Option<Loaded>,
    pub projects: Vec<Summary>,
    pub due: Vec<Due>,
    pub bin: Vec<TrashEntry>,
    pub hits: Vec<Hit>,
    last_find: String,
    pub find: String,
    pub new_project: String,
    pub hide_done: bool,
    // satir metni -> bitti mi: disaridan degisen satirlar amber yansin (Electron'daki snap)
    snap: HashMap<String, bool>,
    pub hot: HashMap<String, Instant>,
    pub hit: Option<String>,
    scroll_to_hit: bool,

    pub edit: Option<project::Edit>,
    pub rename: Option<(String, bool)>,
    pub mem_edit: Option<project::MemEdit>,
    pub doc: Option<project::Doc>,
    pub add: project::Add,

    pub confirm: Option<Confirm>,
    toast: Option<(String, Instant)>,
    pub settings: settings::State,
    pub guide: Option<usize>,
    pub rects: HashMap<&'static str, Rect>,
    focus_add: bool,
    focus_find: bool,
    pub media: HashMap<String, Option<Arc<[u8]>>>,
    last_poll: Instant,
    next_notify: Instant,
}

/// Kucuk kalici bayraklar (rehber goruldu, bitmisleri gizle): Config::home() altinda bos dosya.
pub fn flag(name: &str) -> bool {
    Config::home().join(name).exists()
}

pub fn set_flag(name: &str, on: bool) {
    let f = Config::home().join(name);
    if on {
        let _ = std::fs::create_dir_all(Config::home());
        let _ = std::fs::write(f, "");
    } else {
        let _ = std::fs::remove_file(f);
    }
}

// Sistem dili: Windows'ta kayit defterinden (reg.exe), digerlerinde LANG. Bulunamazsa None → en.
fn system_lang() -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("reg")
            .args(["query", r"HKCU\Control Panel\International", "/v", "LocaleName"])
            .creation_flags(0x0800_0000) // konsol penceresi acilmasin
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.split_whitespace().last().map(String::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("LANG").ok()
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext, store: Store, cfg: Config) -> App {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let store = Arc::new(store);
        let desktop = Desktop::start(cc.egui_ctx.clone(), &cfg, store.dir());
        let shot = shot::Driver::from_args();
        let lang = cfg.lang_code(system_lang().as_deref());
        let pal = theme::palette(&cfg.theme, &cfg.accent, true);
        let mut app = App {
            store,
            cfg,
            desktop,
            i18n: Arc::new(I18n::new(&lang)),
            fonts: theme::Fonts::new(),
            pal,
            applied: Default::default(),
            quick: Arc::new(Mutex::new(quick::State::default())),
            quitting: false,
            visible: true,
            screen: Screen::Welcome,
            cur: None,
            loaded: None,
            projects: vec![],
            due: vec![],
            bin: vec![],
            hits: vec![],
            last_find: String::new(),
            find: String::new(),
            new_project: String::new(),
            hide_done: flag("hide-done"),
            snap: HashMap::new(),
            hot: HashMap::new(),
            hit: None,
            scroll_to_hit: false,
            edit: None,
            rename: None,
            mem_edit: None,
            doc: None,
            add: Default::default(),
            confirm: None,
            toast: None,
            settings: Default::default(),
            guide: None,
            rects: HashMap::new(),
            focus_add: false,
            focus_find: false,
            media: HashMap::new(),
            last_poll: Instant::now(),
            next_notify: Instant::now() + Duration::from_secs(10),
            shot,
            quick_shot_pending: false,
        };
        app.refresh_lists();
        if app.shot.is_none() {
            // son acilan proje: Electron'daki location.hash karsiligi
            if let Some(s) = std::fs::read_to_string(Config::home().join("last-project")).ok().filter(|s| app.projects.iter().any(|p| p.slug == s.trim())) {
                app.open(s.trim());
            }
            app.guide = (!flag("guide-seen")).then_some(0);
        }
        app
    }

    pub fn t(&self, k: &str) -> String {
        self.i18n.t(k, &[])
    }
    pub fn tf(&self, k: &str, args: &[(&str, &str)]) -> String {
        self.i18n.t(k, args)
    }
    pub fn say(&mut self, msg: String) {
        self.toast = Some((msg, Instant::now()));
    }
    /// 409'larin cogu "arada degismis"; ad cakismasi da 409 ama tazelemek cozmez.
    pub fn fail(&mut self, e: notix_core::Error) {
        let msg = if e.code == notix_core::Code::Conflict && e.message != "bu isimde proje var" {
            self.t("changedRefreshed")
        } else {
            self.tf("failed", &[("e", &self.i18n.te(&e))])
        };
        self.say(msg);
        self.reload(false);
    }

    pub fn busy(&self) -> bool {
        self.edit.is_some() || self.rename.is_some() || self.mem_edit.is_some() || self.doc.is_some()
    }
    pub fn searching(&self) -> bool {
        self.find.trim().chars().count() > 1
    }

    pub fn refresh_lists(&mut self) {
        self.projects = self.store.list().unwrap_or_default();
        self.due = self.store.due(&notix_core::today()).unwrap_or_default();
        self.bin = self.store.trash().unwrap_or_default();
        if self.searching() {
            self.hits = self.store.search(self.find.trim()).unwrap_or_default();
        }
    }

    pub fn open(&mut self, slug: &str) {
        if self.cur.as_deref() != Some(slug) {
            self.hit = None;
            self.hot.clear();
            self.snap.clear();
            let _ = std::fs::create_dir_all(Config::home());
            let _ = std::fs::write(Config::home().join("last-project"), slug);
        }
        self.cur = Some(slug.into());
        self.screen = Screen::Project;
        self.reload(false);
    }

    /// Acik projeyi yeniden okur. poll=true: disaridan gelen degisiklik (amber isaret + toast).
    pub fn reload(&mut self, poll: bool) {
        self.refresh_lists();
        let Some(slug) = self.cur.clone() else { return };
        match self.store.parse(&slug) {
            Ok(project) => {
                let tasks: Vec<(&str, bool)> = project
                    .items
                    .iter()
                    .filter_map(|x| if let notix_core::Item::Task { text, done, .. } = x { Some((text.as_str(), *done)) } else { None })
                    .collect();
                if poll {
                    let hot: Vec<String> = tasks.iter().filter(|(t, d)| self.snap.get(*t) != Some(d)).map(|(t, _)| t.to_string()).collect();
                    if !hot.is_empty() {
                        let n = hot.len().to_string();
                        self.say(self.tf("fileChanged", &[("n", &n)]));
                    }
                    for t in hot {
                        self.hot.insert(t, Instant::now());
                    }
                }
                self.snap = tasks.iter().map(|(t, d)| (t.to_string(), *d)).collect();
                let mem = self.store.memory(&slug).unwrap_or(Memory { text: String::new(), from: -1, to: -1 });
                self.loaded = Some(Loaded { project, mem });
            }
            Err(_) => {
                // dosya disaridan silindi/tasindi
                self.loaded = None;
                self.cur = None;
                self.screen = Screen::Welcome;
            }
        }
    }

    pub fn go_hit(&mut self, slug: &str, text: &str) {
        self.find.clear();
        self.open(slug);
        self.hit = Some(text.to_string());
        self.scroll_to_hit = true;
    }

    pub fn take_scroll_to_hit(&mut self) -> bool {
        std::mem::take(&mut self.scroll_to_hit)
    }

    pub fn set_screen(&mut self, s: Screen) {
        self.find.clear();
        self.cur = None;
        self.loaded = None;
        self.screen = s;
        self.refresh_lists();
    }

    /// Ayar degisti: kaydet, masaustune bildir, kisayol baglanamadiysa uyar.
    pub fn save_cfg(&mut self) {
        if let Err(e) = self.cfg.save() {
            self.say(self.tf("failed", &[("e", &e.to_string())]));
        }
        self.store.set_backup(self.cfg.backup);
        if !self.desktop.apply(&self.cfg) {
            self.say(self.tf("keyTaken", &[("k", &self.cfg.quick_add.clone())]));
        }
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let sys = theme::dark_system(ctx);
        let lang = self.cfg.lang_code(system_lang_cached().as_deref());
        let key = (self.cfg.theme.clone(), self.cfg.accent.clone(), sys, lang.clone());
        if key == self.applied {
            return;
        }
        if self.applied.3 != lang {
            self.i18n = Arc::new(I18n::new(&lang));
            self.fonts.install(ctx, &lang);
        }
        self.pal = theme::palette(&self.cfg.theme, &self.cfg.accent, sys);
        theme::apply(ctx, &self.pal);
        self.applied = key;
    }

    // --- uygulama ici kisayollar (find/add/hideDone/help) + yakalama modu ---
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let evs: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        for ev in evs {
            let egui::Event::Key { key, pressed: true, modifiers, repeat: false, .. } = ev else { continue };
            if matches!(key, Key::Escape) && self.settings.cap.is_some() {
                self.settings.cap = None;
                ctx.input_mut(|i| i.events.clear());
                return;
            }
            let Some(a) = accel(key, modifiers) else { continue };
            if let Some(name) = self.settings.cap {
                self.settings.cap = None;
                self.set_key(name, a);
                ctx.input_mut(|i| i.events.clear());
                return;
            }
            if self.guide.is_some() || self.confirm.is_some() || (ctx.egui_wants_keyboard_input() && !modifiers.ctrl && !modifiers.alt) {
                continue;
            }
            let c = &self.cfg;
            let hit = if a == c.find {
                Some(0)
            } else if a == c.add {
                Some(1)
            } else if a == c.hide_done {
                Some(2)
            } else if a == c.help {
                Some(3)
            } else {
                None
            };
            if let Some(h) = hit {
                // yazilacak harf kutuya dusmesin
                ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Text(_))));
                match h {
                    0 => self.focus_find = true,
                    1 => self.focus_add = self.screen == Screen::Project,
                    2 => self.toggle_done(),
                    _ => self.settings.open = true,
                }
                return;
            }
        }
    }

    pub fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub fn toggle_done(&mut self) {
        self.hide_done = !self.hide_done;
        set_flag("hide-done", self.hide_done);
    }

    pub fn focus_add_again(&mut self) {
        self.focus_add = true;
    }

    pub fn take_focus_add(&mut self) -> bool {
        std::mem::take(&mut self.focus_add)
    }

    pub fn rec(&mut self, name: &'static str, r: Rect) {
        self.rects.insert(name, r);
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.desktop.rx.try_recv() {
            match ev {
                Event::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Event::ShowWindow | Event::ShowToday => {
                    self.visible = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    if ev == Event::ShowToday {
                        self.set_screen(Screen::Today);
                    }
                }
                Event::QuickAdd => {
                    quick::open(self, ctx);
                }
                Event::FileChanged(slug) => {
                    if self.cur.as_deref() == Some(slug.as_str()) && !self.busy() {
                        self.reload(true);
                    } else {
                        self.refresh_lists();
                    }
                }
            }
        }
    }
}

// reg.exe her karede cagrilmasin
fn system_lang_cached() -> Option<String> {
    static L: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    L.get_or_init(system_lang).clone()
}

/// egui tusundan Electron accelerator metni ("Control+Alt+N", "?"): yakalama de eslestirme de bunu kullanir.
pub fn accel(key: Key, m: Modifiers) -> Option<String> {
    let name = key.symbol_or_name();
    if name.is_empty() {
        return None;
    }
    let named = name.chars().count() > 1 || name.chars().all(|c| c.is_ascii_alphanumeric());
    let mut v = vec![];
    if m.ctrl {
        v.push("Control".to_string());
    }
    if m.alt {
        v.push("Alt".into());
    }
    if m.shift && named {
        v.push("Shift".into());
    }
    v.push(if name.chars().count() == 1 { name.to_uppercase() } else { name.to_string() });
    Some(v.join("+"))
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.handle_events(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            // kapatmak = tepsiye gizlemek; cikis sadece tepsi menusunden
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.visible = false;
        }
        if Instant::now() >= self.next_notify {
            self.next_notify = Instant::now() + Duration::from_secs(3600);
            if self.shot.is_none() {
                self.desktop.notify_due(&self.store, &self.cfg);
            }
        }
        quick::drive(self, ctx);
        // pencere gizliyken de saatli isler ve olaylar donsun
        ctx.request_repaint_after(Duration::from_secs(5));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _f: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.rects.clear();
        self.sync_theme(&ctx);
        self.shortcuts(&ctx);
        // 5 sn'de bir yedek tazeleme (dosya izleme olayi kacarsa); duzenleme sirasinda ekrani ezme
        if self.last_poll.elapsed() > Duration::from_secs(5) {
            self.last_poll = Instant::now();
            if !self.busy() && !self.settings.open {
                self.reload(true);
            }
        }
        if self.searching() && self.find != self.last_find {
            self.hits = self.store.search(self.find.trim()).unwrap_or_default();
        }
        self.last_find.clone_from(&self.find);
        self.hot.retain(|_, t| t.elapsed() < Duration::from_millis(2400));
        if !self.hot.is_empty() {
            ctx.request_repaint();
        }

        let p = self.pal;
        egui::Panel::left("side")
            .exact_size(250.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).inner_margin(egui::Margin::symmetric(14, 16)))
            .show(ui, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.ink).inner_margin(egui::Margin { left: 30, right: 14, top: 26, bottom: 0 }))
            .show(ui, |ui| self.main_panel(ui));

        self.overlays(&ctx);
        shot::step(self, &ctx);
        ctx.request_repaint_after(Duration::from_secs(5));
    }
}

impl App {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 9.0;
            let (r, _) = ui.allocate_exact_size(vec2(9.0, 9.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 0.0, p.patina);
            ui.label(w::mono("N\u{2009}O\u{2009}T\u{2009}I\u{2009}X", 12.0, p.dim).strong());
        });
        let hint = self.t("newProject");
        let (r, plus, rect) = w::field(ui, &p, &mut self.new_project, &hint, Some("+"));
        self.rec("newp", rect);
        if (plus || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))) && !self.new_project.trim().is_empty() {
            let title = std::mem::take(&mut self.new_project);
            match self.store.create(&title) {
                Ok(slug) => {
                    self.find.clear();
                    self.open(&slug);
                }
                Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
            }
        }
        let hint = self.tf("searchKey", &[("k", &self.cfg.find.clone())]);
        let mut find = std::mem::take(&mut self.find);
        let (fr, _, rect) = w::field(ui, &p, &mut find, &hint, None);
        self.rec("find", rect);
        if self.focus_find {
            self.focus_find = false;
            fr.request_focus();
        }
        if fr.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
            find.clear();
        }
        self.find = find;

        ui.spacing_mut().item_spacing.y = 1.0;
        ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if w::link(ui, &p, &self.t("export"), w::Kind::Plain).clicked() {
                    self.export();
                }
                if w::link(ui, &p, &self.t("import"), w::Kind::Plain).clicked() {
                    self.import();
                }
            });
            let dir = self.store.dir().display().to_string();
            let wh = ui.add(egui::Label::new(w::mono(&dir, 11.0, p.faint)).sense(egui::Sense::click()));
            self.rec("where", wh.rect);
            if wh.on_hover_text(self.t("openDir")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                Desktop::open_folder(self.store.dir());
            }
            let n = if self.bin.is_empty() { String::new() } else { self.bin.len().to_string() };
            if w::proj_row(ui, &p, &self.t("trash"), &n, self.screen == Screen::Trash, self.bin.is_empty()).clicked() {
                self.set_screen(Screen::Trash);
            }
            ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                let n = if self.due.is_empty() { String::new() } else { self.due.len().to_string() };
                let r = w::proj_row(ui, &p, &self.t("today"), &n, self.screen == Screen::Today, self.due.is_empty());
                self.rec("today", r.rect);
                if r.clicked() {
                    self.set_screen(Screen::Today);
                }
                let before = ui.cursor().min;
                egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("projects").show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for s in self.projects.clone() {
                        let n = if s.open > 0 { s.open.to_string() } else { String::new() };
                        let on = self.screen == Screen::Project && self.cur.as_deref() == Some(&s.slug);
                        if w::proj_row(ui, &p, &s.title, &n, on, s.open == 0).clicked() {
                            self.find.clear();
                            self.open(&s.slug);
                        }
                    }
                });
                self.rec("projects", Rect::from_min_size(before, vec2(ui.available_width(), (ui.min_rect().bottom() - before.y).max(40.0))));
            });
        });
    }

    fn main_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt("main").show(ui, |ui| {
            ui.set_max_width(ui.available_width().min(880.0));
            ui.add_space(0.0);
            if self.searching() {
                self.search_view(ui);
            } else if let Some(d) = self.doc.as_ref().map(|d| d.slug.clone()) {
                self.doc_view(ui, &d);
            } else {
                match self.screen {
                    Screen::Project if self.loaded.is_some() => self.project_view(ui),
                    Screen::Today => self.today_view(ui),
                    Screen::Trash => self.trash_view(ui),
                    _ => self.welcome(ui),
                }
            }
            ui.add_space(64.0);
        });
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        let p = self.pal;
        ui.add_space(4.0);
        let r = ui.label(egui::RichText::new(self.t("welcome")).color(p.dim));
        self.rec("empty", r.rect);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(w::mono(&self.cfg.quick_add, 11.0, p.dim).background_color(p.ink));
            ui.label(egui::RichText::new(self.t("welcomeKey")).size(13.0).color(p.faint));
        });
    }

    fn overlays(&mut self, ctx: &egui::Context) {
        let p = self.pal;
        // ayarlar dugmesi (koseden sabit)
        egui::Area::new(egui::Id::unique("corner")).anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -14.0)).order(egui::Order::Middle).show(ctx, |ui| {
            let label = format!("⚙ {}", self.t("settings"));
            let fr = egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(6.0);
            let out = fr.show(ui, |ui| w::link(ui, &p, &label, w::Kind::Plain));
            self.rec("help", out.response.rect);
            if out.inner.on_hover_text(self.tf("settingsKey", &[("k", &self.cfg.help.clone())])).clicked() {
                self.settings.open = true;
            }
        });
        self.confirm_ui(ctx);
        settings::show(self, ctx);
        guide::show(self, ctx);
        // toast
        if let Some((msg, at)) = self.toast.clone() {
            if at.elapsed() > Duration::from_millis(3200) {
                self.toast = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(400));
                egui::Area::new(egui::Id::unique("toast")).anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -60.0)).order(egui::Order::Tooltip).interactable(false).show(ctx, |ui| {
                    ui.set_max_width(340.0);
                    let fr = egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(6.0).inner_margin(egui::Margin::symmetric(12, 9));
                    let out = fr.show(ui, |ui| ui.label(w::mono(msg, 12.5, p.vellum)));
                    let r = out.response.rect;
                    ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(2.0, r.height())), 0.0, p.amber);
                });
            }
        }
    }

    fn confirm_ui(&mut self, ctx: &egui::Context) {
        let Some(c) = self.confirm.as_ref() else { return };
        let msg = match c {
            Confirm::DelLine { text, .. } => self.tf("confirmDel", &[("text", text)]),
            Confirm::TrashProject(s) => self.tf("confirmTrash", &[("slug", s)]),
            Confirm::Purge(b) => self.tf("confirmPurge", &[("title", &b.title), ("file", &b.file)]),
            Confirm::Import(f) => self.tf("confirmImport", &[("n", &f.len().to_string())]),
            Confirm::DiscardDoc => self.t("unsaved"),
        };
        let mut answer = None;
        let m = egui::Modal::new(egui::Id::unique("confirm")).show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.label(msg);
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                    answer = Some(true);
                }
                if ui.button(self.t("cancel")).clicked() {
                    answer = Some(false);
                }
            });
        });
        if m.should_close() {
            answer = Some(false);
        }
        match answer {
            Some(true) => {
                if let Some(c) = self.confirm.take() {
                    self.run_confirm(c);
                }
            }
            Some(false) => self.confirm = None,
            None => {}
        }
    }

    fn run_confirm(&mut self, c: Confirm) {
        match c {
            Confirm::DelLine { i, text } => self.update_task(i, &text, notix_core::TaskUpdate { remove: true, ..Default::default() }),
            Confirm::TrashProject(slug) => match self.store.remove(&slug) {
                Ok(()) => {
                    self.set_screen(Screen::Welcome);
                    self.say(format!("{slug} → notes/.trash"));
                }
                Err(e) => self.fail(e),
            },
            Confirm::Purge(b) => {
                if let Err(e) = self.store.purge(&b.file) {
                    self.say(self.tf("failed", &[("e", &self.i18n.te(&e))]));
                }
                self.refresh_lists();
            }
            Confirm::Import(files) => {
                let (mut n, mut rep) = (0, 0);
                for (slug, body) in &files {
                    match self.store.import_file(slug, body) {
                        Ok(r) => {
                            n += 1;
                            rep += usize::from(r.replaced);
                        }
                        Err(e) => {
                            self.say(self.tf("failed", &[("e", &self.i18n.te(&e))]));
                            return;
                        }
                    }
                }
                let mut m = self.tf("imported", &[("n", &n.to_string())]);
                if rep > 0 {
                    m += &self.tf("importedRep", &[("n", &rep.to_string())]);
                }
                self.say(m);
                self.reload(false);
            }
            Confirm::DiscardDoc => self.doc = None,
        }
    }

    // --- disa/ice aktarma: Electron'la ayni JSON ({version:1, files:{slug:icerik}}) ---
    fn export(&mut self) {
        let files = match self.store.export_all() {
            Ok(f) => f,
            Err(e) => return self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
        };
        let name = format!("notix-{}.json", notix_core::today());
        let Some(path) = rfd::FileDialog::new().set_file_name(name).add_filter("JSON", &["json"]).save_file() else { return };
        let body = serde_json::json!({"version": 1, "files": files});
        match std::fs::write(path, serde_json::to_string_pretty(&body).unwrap_or_default()) {
            Ok(()) => self.say(self.tf("exported", &[("n", &files.len().to_string())])),
            Err(e) => self.say(self.tf("failed", &[("e", &e.to_string())])),
        }
    }

    fn import(&mut self) {
        let Some(paths) = rfd::FileDialog::new().add_filter("JSON / Markdown", &["json", "md"]).pick_files() else { return };
        let mut files = BTreeMap::new();
        for path in paths {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            if path.extension().is_some_and(|e| e == "json") {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                let obj = v.get("files").unwrap_or(&v);
                for (k, v) in obj.as_object().into_iter().flatten() {
                    if let Some(s) = v.as_str() {
                        files.insert(k.clone(), s.to_string());
                    }
                }
            } else if let Some(stem) = path.file_stem() {
                files.insert(stem.to_string_lossy().into_owned(), text);
            }
        }
        if !files.is_empty() {
            self.confirm = Some(Confirm::Import(files));
        }
    }
}
