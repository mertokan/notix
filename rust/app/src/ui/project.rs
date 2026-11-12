// Proje ekrani: baslik, bellek paneli, satirlar, ekleme satiri; belge modu.
use super::md::{self, Seg};
use super::w::{self, col, mono, Kind as LK};
use super::{App, Confirm};
use crate::desktop::Desktop;
use eframe::egui::{
    self, text::{CCursor, CCursorRange}, vec2, Align, FontFamily, FontId, Id, Key, Margin, Modifiers, RichText, Sense, Shape, Stroke, Ui, WidgetInfo, Role,
};
use notix_core::{Dir, Item, Kind, TaskUpdate};

pub struct Edit {
    pub i: usize,
    pub expect: String,
    pub text: String,
    pub fresh: bool,
}
pub struct MemEdit {
    pub text: String,
    pub expect: String,
}
pub struct Doc {
    pub slug: String,
    pub expect: String,
    pub text: String,
}
#[derive(Default)]
pub struct Add {
    pub kind: Kind,
    pub text: String,
}

const OPS_W: f32 = 112.0;
fn bold(s: f32) -> FontId {
    FontId::new(s, FontFamily::Name("bold".into()))
}

fn select_all(ctx: &egui::Context, id: Id, text: &str) {
    let mut st = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0usize), CCursor::new(text.chars().count()))));
    st.store(ctx, id);
}

// --- belge/bellek yazma kisayollari (md.js wrap/prefix) ---
fn byte_at(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

fn sel(ctx: &egui::Context, id: Id, text: &str) -> (usize, usize) {
    let n = text.chars().count();
    let r = egui::text_edit::TextEditState::load(ctx, id).and_then(|s| s.cursor.char_range());
    match r {
        Some(r) => {
            let (a, b) = (r.primary.index.0.min(n), r.secondary.index.0.min(n));
            (a.min(b), a.max(b))
        }
        None => (n, n),
    }
}

fn set_sel(ctx: &egui::Context, id: Id, a: usize, b: usize) {
    let mut st = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
    st.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// Secimi isaretle; secim yoksa imleci isaretin ortasina birakir.
pub fn md_wrap(ctx: &egui::Context, id: Id, text: &mut String, once: &str, sonra: &str) {
    let (a, b) = sel(ctx, id, text);
    let (ba, bb) = (byte_at(text, a), byte_at(text, b));
    text.insert_str(bb, sonra);
    text.insert_str(ba, once);
    let k = once.chars().count();
    set_sel(ctx, id, a + k, b + k);
}

/// Imlecin satirinin basina ekler/kaldirir (baslik, madde).
pub fn md_prefix(ctx: &egui::Context, id: Id, text: &mut String, mark: &str) {
    let (a, _) = sel(ctx, id, text);
    let head: Vec<char> = text.chars().take(a).collect();
    let bas = head.iter().rposition(|c| *c == '\n').map_or(0, |p| p + 1);
    let bb = byte_at(text, bas);
    let k = mark.chars().count();
    if text[bb..].starts_with(mark) {
        text.replace_range(bb..bb + mark.len(), "");
        let n = a.saturating_sub(k).max(bas);
        set_sel(ctx, id, n, n);
    } else {
        text.insert_str(bb, mark);
        set_sel(ctx, id, a + k, a + k);
    }
}

/// Shift'siz Enter'i tuketir (consume_key fazladan Shift'i yok sayar; Shift+Enter alt satir olmali).
pub fn plain_enter(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| {
        let n = i.events.len();
        i.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if !modifiers.shift && !modifiers.ctrl && !modifiers.alt));
        i.events.len() != n
    })
}

/// Ctrl+B/I/K (odaktaki yazi kutusunda)
pub fn md_keys(ui: &Ui, id: Id, text: &mut String) {
    if !ui.memory(|m| m.has_focus(id)) {
        return;
    }
    let ctx = ui.ctx();
    if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::B)) {
        md_wrap(ctx, id, text, "**", "**");
    } else if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::I)) {
        md_wrap(ctx, id, text, "*", "*");
    } else if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::K)) {
        md_wrap(ctx, id, text, "[", "](https://)");
    }
}

fn b64(d: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for c in d.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                o.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

impl App {
    pub fn update_task<'a>(&mut self, i: usize, expect: &'a str, mut u: TaskUpdate<'a>) {
        let Some(slug) = self.cur.clone() else { return };
        u.expect = Some(expect);
        match self.store.update_task(&slug, i, u) {
            Ok(()) => self.reload(false),
            Err(e) => self.fail(e),
        }
    }

    fn move_row(&mut self, i: usize, expect: &str, d: Dir) {
        let Some(slug) = self.cur.clone() else { return };
        match self.store.move_task(&slug, i, d, Some(expect)) {
            Ok(()) => self.reload(false),
            Err(e) => self.fail(e),
        }
    }

    fn move_to(&mut self, i: usize, expect: &str, target: &str, title: &str) {
        let Some(slug) = self.cur.clone() else { return };
        match self.store.move_to_project(&slug, i, target, Some(expect)) {
            Ok(_) => {
                self.reload(false);
                self.say(format!("→ {title}"));
            }
            Err(e) => self.fail(e),
        }
    }

    fn media_bytes(&mut self, url: &str) -> Option<std::sync::Arc<[u8]>> {
        if !self.media.contains_key(url) {
            let b = url
                .strip_prefix("media/")
                .and_then(|n| self.store.media_file(n).ok())
                .and_then(|m| std::fs::read(m.path).ok())
                .map(std::sync::Arc::<[u8]>::from);
            self.media.insert(url.to_string(), b);
        }
        self.media[url].clone()
    }

    pub fn project_view(&mut self, ui: &mut Ui) {
        let p = self.pal;
        let Some(l) = &self.loaded else { return };
        let (project, mem) = (l.project.clone(), l.mem.clone());
        let slug = project.slug.clone();
        let tasks: Vec<bool> = project.items.iter().filter_map(|x| if let Item::Task { done, .. } = x { Some(*done) } else { None }).collect();
        let open = tasks.iter().filter(|d| !**d).count();
        let done = tasks.len() - open;

        // --- baslik ---
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.add(egui::Label::new(mono("#", 21.0, p.faint)).selectable(false));
            let id = Id::unique("rename");
            if let Some((buf, fresh)) = &mut self.rename {
                if std::mem::take(fresh) {
                    select_all(ui.ctx(), id, buf);
                    ui.memory_mut(|m| m.request_focus(id));
                }
                let r = ui.add(egui::TextEdit::singleline(buf).id(id).font(bold(21.0)).desired_width(ui.available_width() - 220.0));
                let esc = ui.input(|i| i.key_pressed(Key::Escape));
                if esc {
                    self.rename = None;
                } else if r.lost_focus() {
                    let title = self.rename.take().map(|x| x.0).unwrap_or_default();
                    if !title.trim().is_empty() && title != project.title {
                        match self.store.rename(&slug, &title) {
                            Ok(n) => self.open(&n),
                            Err(e) => self.fail(e),
                        }
                    }
                }
            } else {
                let r = ui.add(egui::Label::new(RichText::new(&project.title).font(bold(21.0))).sense(Sense::click()).selectable(false));
                if r.on_hover_text(self.t("renameHint")).clicked() {
                    self.rename = Some((project.title.clone(), true));
                }
            }
            let counts = self.tf("counts", &[("open", &self.tf("qOpen", &[("n", &open.to_string())])), ("done", &self.tf("nDone", &[("n", &done.to_string())]))]);
            ui.add(egui::Label::new(mono(counts, 12.0, p.dim)).selectable(false));
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if w::link(ui, &p, &self.t("toTrash"), LK::Warn).on_hover_text(self.t("toTrashHint")).clicked() {
                    self.confirm = Some(Confirm::TrashProject(slug.clone()));
                }
                let lbl = self.t(if self.hide_done { "showDone" } else { "hideDone" });
                if w::link(ui, &p, &lbl, LK::Plain).clicked() {
                    self.toggle_done();
                }
                if w::link(ui, &p, &self.t("doc"), LK::Plain).on_hover_text(self.t("docHint")).clicked() {
                    self.open_doc(&slug);
                }
            });
        });
        let y = ui.cursor().top() + 4.0;
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), y, Stroke::new(1.0, p.rule));
        ui.add_space(14.0);

        self.mem_panel(ui, &mem, &project);

        // --- satirlar ---
        let first_heading = project.items.iter().find(|x| matches!(x, Item::Heading { .. })).map(Item::i);
        for it in &project.items {
            if mem.from >= 0 && (it.i() as i64) >= mem.from && (it.i() as i64) < mem.to {
                continue; // bellek bolumu ustte, listede tekrar gorunmesin
            }
            self.row(ui, it, first_heading);
        }
        self.add_row(ui, &slug);
    }

    fn mem_panel(&mut self, ui: &mut Ui, mem: &notix_core::Memory, project: &notix_core::Project) {
        let p = self.pal;
        let empty = mem.text.is_empty();
        let fr = egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(8.0).inner_margin(Margin { left: 14, right: 14, top: 10, bottom: 12 });
        let out = fr.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let etiket = self.t("memory").to_uppercase();
                ui.add(egui::Label::new(mono(etiket, 11.0, if empty { p.dim } else { p.patina }).strong()).selectable(false));
                if self.mem_edit.is_some() {
                    ui.add(egui::Label::new(mono(self.t("memTools"), 11.0, p.faint)).selectable(false));
                } else {
                    let h = project.items.iter().find(|x| x.i() as i64 == mem.from);
                    let yol = if mem.from >= 0 {
                        let head = match h {
                            Some(Item::Heading { level, text, .. }) => format!("{} {text}", "#".repeat(*level)),
                            _ => "#".into(),
                        };
                        self.tf("memWhere", &[("h", &head), ("n", &(mem.from + 1).to_string())])
                    } else {
                        self.t("memNone")
                    };
                    ui.add(egui::Label::new(mono(yol, 11.0, p.faint)).selectable(false));
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if self.mem_edit.is_some() {
                        if w::link(ui, &p, &self.t("save"), LK::Ok).clicked() {
                            self.save_mem();
                        }
                        if w::link(ui, &p, &self.t("cancel"), LK::Plain).clicked() {
                            self.mem_edit = None;
                        }
                    } else if w::link(ui, &p, &self.t(if empty { "addMem" } else { "edit" }), LK::Plain).clicked() {
                        self.mem_edit = Some(MemEdit { text: mem.text.clone(), expect: mem.text.clone() });
                    }
                });
            });
            ui.add_space(6.0);
            if self.mem_edit.is_some() {
                self.mem_editor(ui);
            } else if empty {
                ui.add(egui::Label::new(RichText::new(self.t("memEmpty")).size(13.0).color(p.faint)));
            } else {
                self.mem_body(ui, &mem.text);
            }
        });
        let r = out.response.rect;
        let bar = if empty { p.rule } else { p.patina };
        ui.painter().rect_filled(egui::Rect::from_min_size(r.min, vec2(2.0, r.height())), egui::CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 }, bar);
        ui.add_space(18.0);
    }

    fn mem_body(&mut self, ui: &mut Ui, text: &str) {
        let p = self.pal;
        let today = notix_core::today();
        ui.spacing_mut().item_spacing.y = 4.0;
        for line in text.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            let t = line.trim_start();
            let hashes = t.bytes().take_while(|b| *b == b'#').count();
            let (body, head, box_) = if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
                (t[hashes + 1..].to_string(), true, None)
            } else if let Some(r) = t.strip_prefix("- [ ]").or_else(|| t.strip_prefix("- [x]")).or_else(|| t.strip_prefix("- [X]")) {
                let done = !t.starts_with("- [ ]");
                (r.trim_start().to_string(), false, Some(done))
            } else {
                (t.strip_prefix("- ").map_or_else(|| line.to_string(), |r| format!("• {r}")), false, None)
            };
            let bitti = box_ == Some(true);
            let mut segs = md::parse(&body);
            if let Some(d) = box_ {
                segs.insert(0, Seg::Text(if d { "[x] " } else { "[ ] " }.into()));
            }
            let st = md::Style { pal: &p, size: 14.0, color: if bitti { p.dim } else { p.vellum }, strike: bitti, bold: head, today: &today };
            let w = ui.available_width();
            let (_, url) = md::rich(ui, md::layout(&segs, &st), w);
            if let Some(u) = url {
                Desktop::open_url(&u);
            }
        }
    }

    fn mem_editor(&mut self, ui: &mut Ui) {
        let id = Id::unique("mem_edit");
        let hint = self.t("memPlaceholder");
        let p = self.pal;
        let Some(m) = &mut self.mem_edit else { return };
        md_keys(ui, id, &mut m.text);
        let out = egui::TextEdit::multiline(&mut m.text)
            .id(id)
            .font(FontId::monospace(13.0))
            .desired_rows(8)
            .desired_width(ui.available_width())
            .hint_text(RichText::new(hint).color(p.faint))
            .show(ui);
        if out.response.has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
            self.mem_edit = None;
        }
    }

    fn save_mem(&mut self) {
        let (Some(m), Some(slug)) = (self.mem_edit.as_ref(), self.cur.clone()) else { return };
        let heading = if self.i18n.lang == "tr" { "Bellek" } else { "Memory" };
        match self.store.set_memory(&slug, &m.text, Some(&m.expect), Some(heading)) {
            Ok(()) => {
                self.mem_edit = None;
                self.reload(false);
                self.say(self.t("memSaved"));
            }
            // 409'da editor acik kalir: yazilan kaybolmasin
            Err(e) if e.code == notix_core::Code::Conflict => self.say(self.t("memConflict")),
            Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
        }
    }

    // --- bir satir: ln | kutu | metin | islemler ---
    fn row(&mut self, ui: &mut Ui, it: &Item, first_heading: Option<usize>) {
        let p = self.pal;
        let (i, text) = (it.i(), it.text().to_string());
        let (is_task, done, indent, level) = match it {
            Item::Task { done, indent, .. } => (true, *done, *indent, 0),
            Item::Heading { level, .. } => (false, false, 0, *level),
            Item::Text { indent, .. } => (false, false, *indent, 0),
        };
        let is_head = level > 0;
        if (is_head && first_heading == Some(i)) || (!is_head && !is_task && text.trim().is_empty()) || (is_task && done && self.hide_done) {
            return;
        }
        let today = notix_core::today();
        let hot = self.hot.get(&text).map(|t| 1.0 - t.elapsed().as_secs_f32() / 2.4);
        let hit = self.hit.as_deref() == Some(text.as_str());
        let hid = Id::unique(("hov", i));
        let hov: bool = ui.data(|d| d.get_temp(hid)).unwrap_or(false);
        let bg = ui.painter().add(Shape::Noop);
        if is_head {
            ui.add_space(12.0);
        }
        let ind = indent.min(16) as f32 * 7.0;
        let editing = self.edit.as_ref().is_some_and(|e| e.i == i);
        let fr = egui::Frame::new().inner_margin(Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let total = ui.available_width();
                col(ui, 26.0, Align::Max, |ui| ui.add(egui::Label::new(mono((i + 1).to_string(), 11.0, p.faint)).selectable(false)));
                let ops_w = if is_task { OPS_W } else { 24.0 };
                let box_w = 27.0 + ind;
                let tw = (total - 26.0 - box_w - ops_w - 30.0).max(80.0);
                col(ui, box_w, Align::Min, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(ind);
                        if is_task {
                            let color = if done { p.patina } else if hov { p.vellum } else { p.dim };
                            let r = ui.add(egui::Label::new(mono(if done { "[x]" } else { "[ ]" }, 14.0, color)).sense(Sense::click()).selectable(false));
                            r.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, done, &text));
                            if r.has_focus() {
                                ui.painter().rect_stroke(r.rect.expand(2.0), 3.0, Stroke::new(2.0, p.patina), egui::StrokeKind::Outside);
                            }
                            if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                self.update_task(i, &text, TaskUpdate { done: Some(!done), ..Default::default() });
                            }
                        } else if is_head {
                            ui.add(egui::Label::new(mono("#".repeat(level), 14.0, p.faint)).selectable(false));
                        }
                    });
                });
                col(ui, tw, Align::Min, |ui| {
                    if editing {
                        self.edit_box(ui, tw);
                    } else {
                        let segs = md::parse(&text);
                        let size = if is_head { if level == 2 { 17.0 } else { 15.0 } } else if is_task { 15.0 } else { 14.0 };
                        let color = if done || (!is_task && !is_head) { p.dim } else { p.vellum };
                        let st = md::Style { pal: &p, size, color, strike: done, bold: is_head, today: &today };
                        let (r, url) = md::rich(ui, md::layout(&segs, &st), tw);
                        if let Some(u) = url {
                            Desktop::open_url(&u);
                        } else if is_task && r.clicked() {
                            self.edit = Some(Edit { i, expect: text.clone(), text: text.clone(), fresh: true });
                        }
                        for s in &segs {
                            if let Seg::Img(alt, url) = s {
                                self.show_image(ui, alt, url, tw);
                            }
                        }
                    }
                });
                col(ui, ops_w, Align::Max, |ui| self.ops(ui, it, hov, &text));
            });
        });
        let rect = fr.response.rect;
        let hovered = ui.rect_contains_pointer(rect);
        if hovered != hov {
            ui.data_mut(|d| d.insert_temp(hid, hovered));
            ui.ctx().request_repaint();
        }
        let strip = |c| Shape::rect_filled(egui::Rect::from_min_size(rect.min, vec2(2.0, rect.height())), 0.0, c);
        let shape = if let Some(f) = hot {
            let f = f.clamp(0.0, 1.0);
            Shape::Vec(vec![Shape::rect_filled(rect, 6.0, super::theme::alpha(p.amber, 0.16 * f)), strip(super::theme::alpha(p.amber, f))])
        } else if hit {
            Shape::Vec(vec![Shape::rect_filled(rect, 6.0, p.hit), strip(p.patina)])
        } else if hov && is_task {
            Shape::rect_filled(rect, 6.0, p.panel)
        } else {
            Shape::Noop
        };
        ui.painter().set(bg, shape);
        if hit && self.take_scroll_to_hit() {
            ui.scroll_to_rect(rect, Some(Align::Center));
        }
    }

    fn show_image(&mut self, ui: &mut Ui, alt: &str, url: &str, w: f32) {
        let p = self.pal;
        match self.media_bytes(url) {
            Some(b) => {
                ui.add_space(6.0);
                let im = egui::Image::from_bytes(format!("bytes://{url}"), egui::load::Bytes::from(b));
                // dar sutunda (max yukseklik 1) otomatik kuculmesin: boyutu kendimiz hesapla
                let sz = im.load_for_size(ui.ctx(), vec2(w, 280.0)).ok().and_then(|t| t.size()).unwrap_or(vec2(64.0, 64.0));
                let k = (w / sz.x).min(280.0 / sz.y).min(1.0);
                ui.add(im.fit_to_exact_size(sz * k).corner_radius(6.0));
            }
            None => {
                // http(s) resimleri yuklenmez (agsiz, bagimliliksiz): etiketi baglanti olarak goster
                let r = ui.add(egui::Label::new(RichText::new(format!("🖼 {}", if alt.is_empty() { url } else { alt })).color(p.patina)).sense(Sense::click()));
                if r.clicked() {
                    Desktop::open_url(url);
                }
            }
        }
    }

    fn edit_box(&mut self, ui: &mut Ui, tw: f32) {
        let p = self.pal;
        let Some(e) = &mut self.edit else { return };
        let id = Id::unique(("edit", e.i));
        if std::mem::take(&mut e.fresh) {
            select_all(ui.ctx(), id, &e.text);
            ui.memory_mut(|m| m.request_focus(id));
        }
        let focused = ui.memory(|m| m.has_focus(id));
        // Enter kaydeder, Shift+Enter alt satir (kutu onu kendisi isler)
        let enter = focused && plain_enter(ui.ctx());
        let r = egui::Frame::new().fill(p.ink).stroke(Stroke::new(1.0, p.patina)).corner_radius(4.0).inner_margin(Margin::symmetric(6, 1)).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut e.text).id(id).frame(egui::Frame::NONE).desired_rows(1).desired_width(tw - 14.0))
        });
        let esc = focused && ui.input(|i| i.key_pressed(Key::Escape));
        if esc {
            self.edit = None;
        } else if enter || r.inner.lost_focus() {
            let Some(e) = self.edit.take() else { return };
            if !e.text.trim().is_empty() && e.text != e.expect {
                self.update_task(e.i, &e.expect, TaskUpdate { text: Some(&e.text), ..Default::default() });
            }
        }
    }

    // ↑ ↓ → ⧉ × : uzerine gelince gorunur (Electron'da opacity); gizliyken de odaklanabilir
    fn ops(&mut self, ui: &mut Ui, it: &Item, hov: bool, text: &str) {
        let p = self.pal;
        let i = it.i();
        let is_task = matches!(it, Item::Task { .. });
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.horizontal(|ui| {
            let btn = |ui: &mut Ui, g: &str, tip: &str, warn: bool| -> egui::Response {
                let (rect, r) = ui.allocate_exact_size(vec2(20.0, 22.0), Sense::click());
                let show = hov || r.has_focus() || r.hovered();
                if show {
                    let c = if r.hovered() { if warn { p.danger } else { p.patina } } else { p.faint };
                    let gal = ui.painter().layout_no_wrap(g.to_string(), FontId::monospace(13.0), c);
                    ui.painter().galley(rect.center() - gal.size() / 2.0, gal, c);
                }
                if r.has_focus() {
                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(2.0, p.patina), egui::StrokeKind::Outside);
                }
                r.widget_info(|| WidgetInfo::labeled(Role::Button, true, tip));
                r.on_hover_text(tip)
            };
            if is_task {
                if btn(ui, "↑", &self.t("up"), false).clicked() {
                    self.move_row(i, text, Dir::Up);
                }
                if btn(ui, "↓", &self.t("down"), false).clicked() {
                    self.move_row(i, text, Dir::Down);
                }
                let r = btn(ui, "→", &self.t("moveTo"), false);
                let list: Vec<_> = self.projects.iter().filter(|s| Some(&s.slug) != self.cur.as_ref()).cloned().collect();
                let mut pick = None;
                egui::Popup::menu(&r).show(|ui| {
                    ui.label(RichText::new(self.t("toProject")).color(p.faint));
                    for s in &list {
                        if ui.button(&s.title).clicked() {
                            pick = Some((s.slug.clone(), s.title.clone()));
                        }
                    }
                });
                if let Some((slug, title)) = pick {
                    egui::Popup::close_all(ui.ctx());
                    self.move_to(i, text, &slug, &title);
                }
            }
            if btn(ui, "⧉", &self.t("copyLine"), false).clicked() {
                ui.ctx().copy_text(text.to_string());
                self.say(self.t("copied"));
            }
            if is_task && btn(ui, "×", &self.t("delLine"), true).clicked() {
                self.confirm = Some(Confirm::DelLine { i, text: text.to_string() });
            }
        });
    }

    // dosyanin bir sonraki satiri: yazarken oraya ne eklendigi gorunur
    fn add_row(&mut self, ui: &mut Ui, slug: &str) {
        let p = self.pal;
        ui.add_space(6.0);
        let y = ui.cursor().top();
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), y, Stroke::new(1.0, p.rule));
        let start = ui.cursor().min;
        let id = Id::unique("add_text");
        let mut submit = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.add_space(8.0 + 26.0);
            let total = ui.available_width();
            let label = |k: Kind| match k {
                Kind::Task => "[ ]",
                Kind::Sub => "↳",
                Kind::Note => "¶",
            };
            egui::ComboBox::from_id_salt("kind").width(27.0).selected_text(mono(label(self.add.kind), 14.0, p.faint)).show_ui(ui, |ui| {
                for k in [Kind::Task, Kind::Sub, Kind::Note] {
                    ui.selectable_value(&mut self.add.kind, k, mono(label(k), 14.0, p.vellum));
                }
            });
            let focused = ui.memory(|m| m.has_focus(id));
            if focused && plain_enter(ui.ctx()) {
                submit = true;
            }
            md_keys(ui, id, &mut self.add.text);
            let hint = RichText::new(self.t("newLine")).color(p.faint);
            let r = ui.add(egui::TextEdit::multiline(&mut self.add.text).id(id).frame(egui::Frame::NONE).desired_rows(1).desired_width(total - 27.0 - 100.0).hint_text(hint));
            if self.take_focus_add() {
                r.request_focus();
            }
            if w::link(ui, &p, &self.t("add"), if focused { LK::Ok } else { LK::Plain }).clicked() {
                submit = true;
            }
        });
        let end = ui.min_rect().bottom();
        self.rec("add", egui::Rect::from_min_max(start, egui::pos2(r.right(), end)));
        if submit && !self.add.text.trim().is_empty() {
            match self.store.add_task(slug, &self.add.text, self.add.kind) {
                Ok(()) => {
                    self.add.text.clear();
                    self.reload(false);
                    self.focus_add_again();
                }
                Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
            }
        }
    }

    // --- belge modu ---
    pub fn open_doc(&mut self, slug: &str) {
        match self.store.read(slug) {
            Ok(text) => self.doc = Some(Doc { slug: slug.into(), expect: text.clone(), text }),
            Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
        }
    }

    pub fn doc_view(&mut self, ui: &mut Ui, slug: &str) {
        let p = self.pal;
        let id = Id::unique("doc_text");
        let mut act = 0; // 1 vazgec, 2 kaydet
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.add(egui::Label::new(mono("#", 21.0, p.faint)).selectable(false));
            ui.add(egui::Label::new(RichText::new(slug).font(bold(21.0))).selectable(false));
            ui.add(egui::Label::new(mono(self.t("docMeta"), 12.0, p.dim)).selectable(false));
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if w::link(ui, &p, &self.t("save"), LK::Ok).clicked() {
                    act = 2;
                }
                if w::link(ui, &p, &self.t("cancel"), LK::Plain).clicked() {
                    act = 1;
                }
            });
        });
        let y = ui.cursor().top() + 4.0;
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), y, Stroke::new(1.0, p.rule));
        ui.add_space(14.0);
        let ctx = ui.ctx().clone();
        let Some(d) = self.doc.as_mut() else { return };
        let mut dtext = std::mem::take(&mut d.text);
        let tr = self.i18n.clone();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
            let tool = |ui: &mut Ui, l: &str, tip: String| w::link(ui, &p, l, LK::Plain).on_hover_text(tip).clicked();
            let doc = &mut dtext;
            if tool(ui, "H1", tr.t("tH1", &[])) {
                md_prefix(&ctx, id, doc, "# ");
            }
            if tool(ui, "H2", tr.t("tH2", &[])) {
                md_prefix(&ctx, id, doc, "## ");
            }
            if tool(ui, "B", tr.t("tB", &[])) {
                md_wrap(&ctx, id, doc, "**", "**");
            }
            if tool(ui, "I", tr.t("tI", &[])) {
                md_wrap(&ctx, id, doc, "*", "*");
            }
            if tool(ui, &tr.t("tCodeL", &[]), tr.t("tCode", &[])) {
                md_wrap(&ctx, id, doc, "`", "`");
            }
            if tool(ui, "•", tr.t("tList", &[])) {
                md_prefix(&ctx, id, doc, "- ");
            }
            if tool(ui, "[ ]", tr.t("tTodo", &[])) {
                md_prefix(&ctx, id, doc, "- [ ] ");
            }
            if tool(ui, &tr.t("tLinkL", &[]), tr.t("tLink", &[])) {
                md_wrap(&ctx, id, doc, "[", "](https://)");
            }
            if tool(ui, &tr.t("tImgL", &[]), tr.t("tImg", &[])) {
                if let Some(f) = rfd::FileDialog::new().add_filter("image", &["png", "jpg", "jpeg", "gif", "webp", "svg"]).pick_file() {
                    let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    match std::fs::read(&f).map_err(notix_core::Error::from).and_then(|b| self.store.save_media(&name, &b64(&b))) {
                        Ok(path) => {
                            let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
                            md_wrap(&ctx, id, doc, &format!("![{stem}]({path})"), "");
                        }
                        Err(e) => {
                            let m = self.tf("imgFailed", &[("e", &self.i18n.te(&e))]);
                            self.toast = Some((m, std::time::Instant::now()));
                        }
                    }
                }
            }
        });
        md_keys(ui, id, &mut dtext);
        let h = (ui.available_height() - 60.0).max(200.0);
        egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(8.0).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("docscroll").max_height(h).show(ui, |ui| {
                egui::TextEdit::multiline(&mut dtext)
                    .id(id)
                    .frame(egui::Frame::NONE)
                    .font(FontId::monospace(13.5))
                    .desired_width(ui.available_width())
                    .desired_rows((h / 22.0) as usize)
                    .margin(Margin::symmetric(16, 14))
                    .show(ui)
            });
        });
        if let Some(d) = self.doc.as_mut() {
            d.text = dtext;
        }
        ui.add_space(8.0);
        ui.label(RichText::new(self.t("docHint2")).size(13.0).color(p.faint));
        match act {
            1 => {
                if self.doc.as_ref().is_some_and(|d| d.text != d.expect) {
                    self.confirm = Some(Confirm::DiscardDoc);
                } else {
                    self.doc = None;
                }
            }
            2 => self.save_doc(),
            _ => {}
        }
    }

    pub fn save_doc_for_test(&mut self) {
        self.save_doc()
    }
    pub fn save_mem_for_test(&mut self) {
        self.save_mem()
    }

    fn save_doc(&mut self) {
        let Some(d) = &self.doc else { return };
        match self.store.write(&d.slug, &d.text, Some(&d.expect)) {
            Ok(()) => {
                self.doc = None;
                self.reload(false);
                self.say(self.t("docSaved"));
            }
            // 409'da editor kapanmaz
            Err(e) if e.code == notix_core::Code::Conflict => self.say(self.t("docConflict")),
            Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
        }
    }
}
