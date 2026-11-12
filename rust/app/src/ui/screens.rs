// Proje disi ekranlar: arama sonuclari, Bugun, Cop kutusu.
use super::md;
use super::w::{self, mono, Kind as LK};
use super::{App, Confirm};
use eframe::egui::{self, Align, FontFamily, FontId, Id, Margin, RichText, Sense, Shape, Stroke, Ui};

/// ms (epoch) → yerel "YYYY-MM-DD SS:DD" (Electron toLocaleString gibi kullanicinin saati).
fn when(ms: f64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

impl App {
    pub fn page_header(&self, ui: &mut Ui, title: &str, meta: &str) {
        let p = self.pal;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.add(egui::Label::new(mono("#", 21.0, p.faint)).selectable(false));
            ui.add(egui::Label::new(RichText::new(title).font(FontId::new(21.0, FontFamily::Name("bold".into())))).selectable(false));
            ui.add(egui::Label::new(mono(meta, 12.0, p.dim)).selectable(false));
        });
        let y = ui.cursor().top() + 4.0;
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), y, Stroke::new(1.0, p.rule));
        ui.add_space(14.0);
    }

    /// Tiklanabilir sonuc satiri: [yer | kutu | metin]; tiklayinca true.
    /// flags: (bitti gibi soluk, markdown goster)
    fn hit_row(&mut self, ui: &mut Ui, id: Id, where_: &str, boxed: &str, text: &str, flags: (bool, bool)) -> bool {
        let (off, rich) = flags;
        let p = self.pal;
        let today = notix_core::today();
        let bg = ui.painter().add(Shape::Noop);
        let fr = egui::Frame::new().inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let total = ui.available_width();
                w::col(ui, 110.0, Align::Max, |ui| {
                    let mut job = egui::text::LayoutJob::simple_singleline(where_.to_string(), FontId::monospace(11.0), p.faint);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(110.0);
                    ui.add(egui::Label::new(job).selectable(false));
                });
                w::col(ui, 27.0, Align::Min, |ui| ui.add(egui::Label::new(mono(boxed, 14.0, p.dim)).selectable(false)));
                w::col(ui, (total - 110.0 - 27.0 - 20.0).max(80.0), Align::Min, |ui| {
                    let segs = if rich { md::parse(text) } else { vec![md::Seg::Text(text.to_string())] };
                    let st = md::Style { pal: &p, size: 15.0, color: if off { p.dim } else { p.vellum }, strike: off, bold: false, today: &today };
                    let w = ui.available_width();
                    let mut job = md::layout(&segs, &st).job;
                    job.wrap.max_width = w;
                    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Wrap).selectable(false));
                });
            });
        });
        let rect = fr.response.rect;
        let r = ui.interact(rect, id, Sense::click());
        if r.hovered() {
            ui.painter().set(bg, Shape::rect_filled(rect, 6.0, p.panel));
        }
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, text));
        r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }

    pub fn search_view(&mut self, ui: &mut Ui) {
        let p = self.pal;
        let q = self.find.trim().to_string();
        self.page_header(ui, &format!("“{q}”"), &self.tf("lines", &[("n", &self.hits.len().to_string())]));
        if self.hits.is_empty() {
            ui.label(RichText::new(self.t("noHits")).color(p.dim));
            return;
        }
        let mut go = None;
        for (k, h) in self.hits.clone().iter().enumerate() {
            let b = if h.kind == "task" { if h.done == Some(true) { "[x]" } else { "[ ]" } } else { "¶" };
            if self.hit_row(ui, Id::unique(("hit", k)), &h.project, b, &h.text, (h.done == Some(true), false)) {
                go = Some((h.slug.clone(), h.text.clone()));
            }
        }
        if let Some((s, t)) = go {
            self.go_hit(&s, &t);
        }
    }

    pub fn today_view(&mut self, ui: &mut Ui) {
        let p = self.pal;
        let meta = self.tf("todayMeta", &[("d", &notix_core::today()), ("n", &self.due.len().to_string())]);
        self.page_header(ui, &self.t("today"), &meta);
        if self.due.is_empty() {
            ui.label(RichText::new(self.t("todayEmpty")).color(p.dim));
        }
        let mut go = None;
        for (k, h) in self.due.clone().iter().enumerate() {
            if self.hit_row(ui, Id::unique(("due", k)), &h.project, "[ ]", &h.text, (false, true)) {
                go = Some((h.slug.clone(), h.text.clone()));
            }
        }
        if let Some((s, t)) = go {
            self.go_hit(&s, &t);
        }
        ui.add_space(14.0);
        ui.label(RichText::new(self.t("todayHint")).size(13.0).color(p.faint));
    }

    pub fn trash_view(&mut self, ui: &mut Ui) {
        let p = self.pal;
        let meta = self.tf("trashMeta", &[("n", &self.bin.len().to_string())]);
        self.page_header(ui, &self.t("trash"), &meta);
        if self.bin.is_empty() {
            ui.label(RichText::new(self.t("trashEmpty")).color(p.dim));
        }
        let (mut restore, mut purge) = (None, None);
        for b in self.bin.clone() {
            egui::Frame::new().inner_margin(Margin::symmetric(8, 7)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    w::col(ui, 116.0, Align::Min, |ui| ui.add(egui::Label::new(mono(when(b.at), 11.0, p.faint)).selectable(false)));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(&b.title);
                        let meta = self.tf(
                            "trashFile",
                            &[("file", &b.file), ("open", &self.tf("qOpen", &[("n", &b.open.to_string())])), ("done", &self.tf("nDone", &[("n", &b.done.to_string())]))],
                        );
                        ui.label(mono(meta, 11.0, p.faint));
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if w::link(ui, &p, &self.t("purge"), LK::Warn).clicked() {
                            purge = Some(b.clone());
                        }
                        if w::link(ui, &p, &self.t("restore"), LK::Plain).clicked() {
                            restore = Some(b.clone());
                        }
                    });
                });
            });
        }
        if let Some(b) = restore {
            match self.store.restore(&b.file) {
                Ok(slug) => {
                    let stem = b.file.strip_suffix(".md").unwrap_or(&b.file);
                    let digits = stem.bytes().rev().take_while(u8::is_ascii_digit).count();
                    let base = stem[..stem.len() - digits].strip_suffix('-').filter(|_| digits > 0).unwrap_or(stem);
                    let key = if slug == base { "restored" } else { "restoredAs" };
                    self.say(self.tf(key, &[("slug", &slug)]));
                    self.refresh_lists();
                }
                Err(e) => self.say(self.tf("failed", &[("e", &self.i18n.te(&e))])),
            }
        }
        if let Some(b) = purge {
            self.confirm = Some(Confirm::Purge(b));
        }
        ui.add_space(14.0);
        ui.label(RichText::new(self.t("trashHint")).size(13.0).color(p.faint));
    }
}

