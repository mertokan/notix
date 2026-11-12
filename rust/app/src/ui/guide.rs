// Ilk acilis rehberi (public/app.js rehber()): vurgulanan oge halkayla, gerisi kararir.
use super::{set_flag, theme, App};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Key, Margin, Modifiers, Rect, RichText, Stroke};

// (baslik, govde, hedefler): hedeflerden ilk gorunen vurgulanir; hicbiri yoksa kart ortada.
const STEPS: [(&str, &str, &[&str]); 7] = [
    ("g1t", "g1b", &[]),
    ("g2t", "g2b", &["newp"]),
    ("g3t", "g3b", &["add", "projects", "empty"]),
    ("today", "g4b", &["today"]),
    ("g5t", "g5b", &["find"]),
    ("settings", "g6b", &["help"]),
    ("ai", "g7b", &["where"]),
];

pub fn restart(app: &mut App) {
    app.guide = Some(0);
}

fn finish(app: &mut App) {
    set_flag("guide-seen", true);
    app.guide = None;
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut i) = app.guide else { return };
    let p = app.pal;
    let screen = ctx.content_rect();
    let last = STEPS.len() - 1;
    let mut act = 0i32; // 1 ileri, -1 geri, 2 atla
    ctx.input_mut(|inp| {
        if inp.consume_key(Modifiers::NONE, Key::Escape) {
            act = 2;
        } else if inp.consume_key(Modifiers::NONE, Key::ArrowRight) {
            act = 1;
        } else if inp.consume_key(Modifiers::NONE, Key::ArrowLeft) {
            act = -1;
        }
    });
    let (title, body, targets) = STEPS[i];
    let hole = targets.iter().find_map(|n| app.rects.get(n)).copied().filter(|r| r.width() > 1.0).map(|r| r.expand(6.0));

    // kart konumu: once hedefin sagi, sigmazsa alti, o da sigmazsa ustu
    let size: egui::Vec2 = ctx.data(|d| d.get_temp(egui::Id::unique("guide_size"))).unwrap_or(vec2(340.0, 190.0));
    let m = 12.0;
    let area = egui::Modal::default_area(egui::Id::unique("guide"));
    let area = match hole {
        Some(h) => {
            let (mut x, mut y) = (h.right() + m, h.top());
            if x + size.x > screen.right() - m {
                (x, y) = (h.left(), h.bottom() + m);
            }
            if y + size.y > screen.bottom() - m {
                y = h.top() - size.y - m;
            }
            area.anchor(Align2::LEFT_TOP, vec2(x.clamp(m, screen.right() - size.x - m), y.clamp(m, screen.bottom() - size.y - m)))
        }
        None => area,
    };
    let card = egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(10.0).inner_margin(Margin { left: 18, right: 18, top: 16, bottom: 12 });
    let out = egui::Modal::new(egui::Id::unique("guide")).backdrop_color(Color32::TRANSPARENT).area(area).frame(egui::Frame::NONE).show(ctx, |ui| {
        // karartma: halka cevresi acik, gerisi %60 siyah
        let dark = Color32::from_black_alpha(153);
        let pt = ui.painter();
        match hole {
            Some(h) => {
                pt.rect_filled(Rect::from_min_max(screen.min, pos2(screen.right(), h.top())), 0.0, dark);
                pt.rect_filled(Rect::from_min_max(pos2(screen.left(), h.bottom()), screen.max), 0.0, dark);
                pt.rect_filled(Rect::from_min_max(pos2(screen.left(), h.top()), pos2(h.left(), h.bottom())), 0.0, dark);
                pt.rect_filled(Rect::from_min_max(pos2(h.right(), h.top()), pos2(screen.right(), h.bottom())), 0.0, dark);
                pt.rect_stroke(h.expand(4.0), 10.0, Stroke::new(4.0, p.sel), egui::StrokeKind::Outside);
                pt.rect_stroke(h, 8.0, Stroke::new(2.0, p.patina), egui::StrokeKind::Outside);
            }
            None => {
                pt.rect_filled(screen, 0.0, dark);
            }
        }
        card.show(ui, |ui| {
            ui.set_width(304.0);
            ui.add(egui::Label::new(super::w::mono(format!("{}/{}", i + 1, STEPS.len()), 11.0, p.faint)).selectable(false));
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(app.t(title)).font(egui::FontId::new(15.0, egui::FontFamily::Name("bold".into())))));
            ui.add_space(4.0);
            let args = [("k", app.cfg.quick_add.as_str()), ("s", &app.t("settings")), ("m", &app.t("copyMcp"))];
            let args: Vec<(&str, &str)> = args.iter().map(|(a, b)| (*a, *b)).collect();
            ui.label(RichText::new(app.i18n.t(body, &args)).size(13.5).color(p.dim));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if i != last && super::w::link(ui, &p, &app.t("gSkip"), super::w::Kind::Plain).clicked() {
                    act = 2;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let next = egui::Button::new(RichText::new(app.t(if i == last { "gStart" } else { "gNext" })).color(p.ink).strong()).fill(p.patina);
                    let r = ui.add(next);
                    // adim degisince odak "ileri"de
                    let seen: Option<usize> = ui.data(|d| d.get_temp(egui::Id::unique("guide_step")));
                    if seen != Some(i) {
                        r.request_focus();
                        ui.data_mut(|d| d.insert_temp(egui::Id::unique("guide_step"), i));
                    }
                    if r.clicked() {
                        act = 1;
                    }
                    if i > 0 && super::w::link(ui, &p, &app.t("gBack"), super::w::Kind::Plain).clicked() {
                        act = -1;
                    }
                });
            });

        });
    });
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique("guide_size"), out.response.rect.size()));
    let _ = theme::alpha;
    match act {
        1 if i == last => finish(app),
        1 => i += 1,
        -1 if i > 0 => i -= 1,
        2 => finish(app),
        _ => {}
    }
    if app.guide.is_some() {
        app.guide = Some(i);
    }
}
