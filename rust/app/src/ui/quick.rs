// Hizli ekleme kutusu (public/quick.html): cercevesiz, hep ustte, gorev cubugunda yok.
// Ana pencere gizliyken de acilsin diye viewport logic()'ten, deferred olarak cizilir.
use super::i18n::I18n;
use super::theme::Pal;
use super::w::mono;
use super::App;
use crate::config::Config;
use eframe::egui::{self, vec2, Key, Modifiers, RichText, Stroke, ViewportCommand};
use notix_core::{Kind, Store};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const BAR: f32 = 62.0;
const ROW: f32 = 34.0;
const MAX_ROWS: usize = 6;
const WIDTH: f32 = 560.0;

#[derive(Default)]
pub struct State {
    pub open: bool,
    text: String,
    idx: usize,
    /// (slug, baslik, acik sayisi); Inbox hep ilk
    projects: Vec<(String, String, usize)>,
    list: bool,
    err: Option<String>,
    fresh: bool,
    was_focused: bool,
    done: bool,
    opened: Option<Instant>,
    pal: Option<Pal>,
    /// --shot: ekran goruntusu yolu + bitti bayragi
    pub shot: Option<(std::path::PathBuf, Arc<AtomicBool>)>,
    frames: u32,
}

fn id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("notix-quick")
}

pub fn open(app: &mut App, ctx: &egui::Context) {
    let all = app.store.list().unwrap_or_default();
    let mut projects = vec![("inbox".to_string(), "inbox".to_string(), 0)];
    for s in &all {
        let row = (s.slug.clone(), s.title.clone(), s.open);
        if s.slug == "inbox" {
            projects[0] = row;
        } else {
            projects.push(row);
        }
    }
    let last = std::fs::read_to_string(Config::home().join("quick-to")).unwrap_or_default();
    let mut s = app.quick.lock().unwrap();
    s.idx = projects.iter().position(|p| p.0 == last.trim()).unwrap_or(0);
    s.projects = projects;
    s.text.clear();
    (s.open, s.list, s.err, s.fresh, s.was_focused, s.done, s.frames) = (true, false, None, true, false, false, 0);
    s.opened = Some(Instant::now());
    drop(s);
    // ana pencere gizliyken ikinci viewport'un cizilmesi icin
    ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
    ctx.request_repaint();
}

/// logic()'ten her karede: acik oldugu surece viewport'u (yeniden) bildirir.
pub fn drive(app: &mut App, ctx: &egui::Context) {
    let (store, i18n, st) = (app.store.clone(), app.i18n.clone(), app.quick.clone());
    {
        let mut s = st.lock().unwrap();
        if !s.open {
            return;
        }
        if s.done {
            s.open = false;
            drop(s);
            ctx.request_repaint(); // viewport bu karede bildirilmeyince kapanir
            return;
        }
        s.pal = Some(app.pal);
    }
    let mon = ctx.input(|i| i.viewport().monitor_size).unwrap_or(vec2(1920.0, 1080.0));
    let builder = egui::ViewportBuilder::default()
        .with_title(app.t("qTitle"))
        .with_inner_size([WIDTH, BAR])
        .with_position(egui::pos2((mon.x - WIDTH) / 2.0, mon.y * 0.22))
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_taskbar(false);
    ctx.show_viewport_deferred(id(), builder, move |ui, _| draw(ui, &st, &store, &i18n));
}

fn draw(ui: &mut egui::Ui, st: &Arc<Mutex<State>>, store: &Store, tr: &I18n) {
    let mut s = st.lock().unwrap();
    let Some(p) = s.pal else { return };
    let ctx = ui.ctx().clone();
    s.frames += 1;
    // odak kaybolunca kapan (ilk odak gelmeden degil)
    match ctx.input(|i| i.viewport().focused) {
        Some(true) => s.was_focused = true,
        Some(false) if s.was_focused && s.opened.is_some_and(|t| t.elapsed().as_millis() > 400) => s.done = true,
        _ => {}
    }
    if ctx.input(|i| i.viewport().close_requested()) {
        s.done = true;
    }
    if s.fresh {
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }
    let n = s.projects.len();
    let border = if s.err.is_some() { p.danger } else { p.patina };
    let mut grow = None;
    egui::CentralPanel::default().frame(egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, border))).show(ui, |ui| {
        let box_id = egui::Id::unique("quick-box");
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            if s.list {
                s.list = false;
                grow = Some(BAR);
            } else {
                s.done = true;
            }
        }
        // listeyi acmadan proje degistir
        let step = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) as i32 - ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) as i32;
        if step != 0 && n > 0 {
            s.idx = (s.idx as i32 + step).rem_euclid(n as i32) as usize;
            let _ = std::fs::create_dir_all(Config::home());
            let _ = std::fs::write(Config::home().join("quick-to"), &s.projects[s.idx].0);
        }
        let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        ui.allocate_ui(vec2(WIDTH - 2.0, BAR - 2.0), |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.add(egui::Label::new(mono("[ ]", 15.0, border)).selectable(false));
                let name = s.projects.get(s.idx).map(|x| x.1.clone()).unwrap_or_default();
                let pick_w = (name.chars().count() as f32 * 7.5 + 50.0).min(210.0);
                let hint = RichText::new(tr.t("qPlaceholder", &[])).color(p.faint).size(16.0);
                let r = ui.add(
                    egui::TextEdit::singleline(&mut s.text)
                        .id(box_id)
                        .frame(egui::Frame::NONE)
                        .font(egui::FontId::proportional(16.0))
                        .desired_width(ui.available_width() - pick_w - 24.0)
                        .hint_text(hint),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, tr.t("qAria", &[])));
                if s.fresh {
                    r.request_focus();
                    s.fresh = false;
                }
                let label = format!("⇅ {name}");
                let b = ui.add(egui::Button::new(mono(label, 12.0, if s.list { p.patina } else { p.faint })).frame(false).fill(if s.list { p.raise } else { egui::Color32::TRANSPARENT }));
                b.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr.t("qPick", &[])));
                if b.on_hover_text(tr.t("qPick", &[])).clicked() {
                    s.list = !s.list;
                    grow = Some(if s.list { BAR + n.min(MAX_ROWS) as f32 * ROW + 12.0 } else { BAR });
                    r.request_focus();
                }
                if enter && !s.text.trim().is_empty() {
                    submit(&mut s, store, tr);
                }
            });
        });
        if s.list {
            let r = ui.available_rect_before_wrap();
            ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.rule));
            egui::ScrollArea::vertical().max_height(n.min(MAX_ROWS) as f32 * ROW + 6.0).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for k in 0..n {
                    let (slug, title, open) = s.projects[k].clone();
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), egui::Sense::click());
                    if k == s.idx || resp.hovered() {
                        ui.painter().rect_filled(rect.shrink2(vec2(6.0, 0.0)), 6.0, p.raise);
                    }
                    let c = if k == s.idx { p.patina } else { p.vellum };
                    ui.painter().text(rect.left_center() + vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, &title, egui::FontId::proportional(13.5), c);
                    if open > 0 {
                        let t = tr.t("qOpen", &[("n", &open.to_string())]);
                        ui.painter().text(rect.right_center() - vec2(16.0, 0.0), egui::Align2::RIGHT_CENTER, t, egui::FontId::monospace(11.0), p.faint);
                    }
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &title));
                    if resp.clicked() {
                        s.idx = k;
                        s.list = false;
                        grow = Some(BAR);
                        s.fresh = true;
                        let _ = std::fs::write(Config::home().join("quick-to"), slug);
                    }
                }
            });
        }
    });
    if let Some(h) = grow {
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(WIDTH, h)));
    }
    if let Some((path, flag)) = s.shot.clone() {
        if s.frames > 12 {
            s.shot = None;
            ctx.request_screenshot(move |img| {
                let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let _ = image::save_buffer(path, &raw, img.size[0] as u32, img.size[1] as u32, image::ColorType::Rgba8);
                flag.store(true, Ordering::SeqCst);
            });
        }
        ctx.request_repaint();
    }
    ctx.request_repaint_of(egui::ViewportId::ROOT);
}

fn submit(s: &mut State, store: &Store, tr: &I18n) {
    let slug = s.projects[s.idx].0.clone();
    // Inbox yoksa acilir; create var olan dosyaya dokunmaz
    let r = (if slug == "inbox" { store.create("Inbox").map(|_| ()) } else { Ok(()) }).and_then(|_| store.add_task(&slug, &s.text, Kind::Task));
    match r {
        Ok(()) => (s.err, s.done) = (None, true),
        // yazilan satir kutuda kalsin, kaybolmasin
        Err(e) => {
            s.err = Some(tr.te(&e));
            s.fresh = true;
        }
    }
}
