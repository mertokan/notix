// Kucuk ortak parcalar: .link dugmesi, kenar sutunu satiri, kutu cercevesi (style.css karsiliklari).
use super::theme::Pal;
use eframe::egui::{self, vec2, Align, Color32, FontId, Layout, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, Role};

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Plain,
    Warn,
    Ok,
}

/// `.link`: duz, mono 12, uzerine gelince arka plan. Etiket AccessKit'e dugme olarak gider.
pub fn link(ui: &mut Ui, p: &Pal, text: &str, kind: Kind) -> Response {
    let font = FontId::monospace(12.0);
    let base = if kind == Kind::Ok { p.patina } else { p.faint };
    let size = ui.painter().layout_no_wrap(text.to_string(), font.clone(), base).size() + vec2(14.0, 10.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let on = resp.hovered() || resp.has_focus();
    let (bg, fg) = match (on, kind) {
        (true, Kind::Warn) => (p.danger_bg, p.danger),
        (true, Kind::Ok) => (p.raise, p.patina),
        (true, Kind::Plain) => (p.raise, p.vellum),
        _ => (Color32::TRANSPARENT, base),
    };
    ui.painter().rect_filled(rect, 6.0, bg);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    ui.painter().galley(rect.center() - g.size() / 2.0, g, fg);
    if resp.has_focus() {
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(2.0, p.patina), StrokeKind::Outside);
    }
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, text));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Kenar sutunu satiri (.proj): ad solda, sayac sagda (mono, vurgu rengi).
pub fn proj_row(ui: &mut Ui, p: &Pal, name: &str, count: &str, on: bool, clear: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 31.0), Sense::click());
    if on || resp.hovered() || resp.has_focus() {
        ui.painter().rect_filled(rect, 6.0, p.raise);
    }
    if on {
        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, vec2(2.0, rect.height())), 0.0, p.patina);
    }
    let cnt = ui.painter().layout_no_wrap(count.to_string(), FontId::monospace(12.0), p.patina);
    let cw = if count.is_empty() { 0.0 } else { cnt.size().x + 8.0 };
    let col = if clear { p.dim } else { p.vellum };
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), FontId::proportional(14.0), col);
    job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 18.0 - cw);
    let g = ui.painter().layout_job(job);
    ui.painter().galley(egui::pos2(rect.left() + 9.0, rect.center().y - g.size().y / 2.0), g, col);
    ui.painter().galley(egui::pos2(rect.right() - 9.0 - cnt.size().x, rect.center().y - cnt.size().y / 2.0), cnt, p.patina);
    if resp.has_focus() {
        ui.painter().rect_stroke(rect, 6.0, Stroke::new(2.0, p.patina), StrokeKind::Inside);
    }
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, format!("{name} {count}")));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Cerceveli metin kutusu (#newp / #find): odakta kenar vurgu rengi.
pub fn field(ui: &mut Ui, p: &Pal, text: &mut String, hint: &str, trailing: Option<&str>) -> (Response, bool, egui::Rect) {
    let mut plus = false;
    let fr = egui::Frame::new().fill(p.ink).stroke(Stroke::new(1.0, p.rule)).corner_radius(6.0).inner_margin(egui::Margin::symmetric(9, 3));
    let out = fr.show(ui, |ui| {
        ui.horizontal(|ui| {
            let w = ui.available_width() - if trailing.is_some() { 26.0 } else { 0.0 };
            let r = ui.add(
                egui::TextEdit::singleline(text)
                    .frame(egui::Frame::NONE)
                    .desired_width(w)
                    .margin(egui::Margin::symmetric(0, 4))
                    .hint_text(egui::RichText::new(hint).color(p.faint)),
            );
            if let Some(t) = trailing {
                let b = ui.add(egui::Button::new(egui::RichText::new(t).monospace().size(16.0).color(p.faint)).frame(false).min_size(vec2(20.0, 20.0)));
                plus = b.clicked();
                b.widget_info(|| WidgetInfo::labeled(Role::Button, true, hint));
            }
            r
        })
        .inner
    });
    if out.inner.has_focus() {
        ui.painter().rect_stroke(out.response.rect, 6.0, Stroke::new(1.0, p.patina), StrokeKind::Inside);
    }
    (out.inner, plus, out.response.rect)
}

/// Sabit genislikli sutun (satirlardaki ln/kutu/ops hucreleri).
pub fn col<R>(ui: &mut Ui, w: f32, align: Align, f: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(vec2(w, 1.0), Layout::top_down(align), |ui| {
        ui.set_width(w);
        f(ui)
    })
    .inner
}

pub fn mono(s: impl Into<String>, size: f32, color: Color32) -> egui::RichText {
    egui::RichText::new(s).font(FontId::monospace(size)).color(color)
}
