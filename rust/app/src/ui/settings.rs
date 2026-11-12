// Ayarlar penceresi: gorunum, kisayollar, uygulama, AI ajanlari (Electron'daki showKeys).
use super::w::{self, mono, Kind as LK};
use super::{guide, i18n, theme, App};
use crate::config::{ACCENTS, LANGS, THEMES};
use crate::desktop::Desktop;
use eframe::egui::{self, vec2, Align, Margin, RichText, Sense, Stroke, Ui};

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// hangi kisayol yakalama modunda
    pub cap: Option<&'static str>,
    /// klasor degisti, yeniden baslatma bekliyor
    pub dir_note: bool,
    pub mcp_copied: bool,
}

const KEYS: [(&str, &str); 5] = [("quickAdd", "kQuickAdd"), ("find", "kFind"), ("add", "kAdd"), ("hideDone", "kHideDone"), ("help", "kHelp")];

fn h3(ui: &mut Ui, p: &theme::Pal, text: &str, first: bool) {
    if !first {
        ui.add_space(14.0);
        let r = ui.available_rect_before_wrap();
        ui.painter().hline(r.x_range(), ui.cursor().top(), Stroke::new(1.0, p.rule));
        ui.add_space(14.0);
    }
    ui.add(egui::Label::new(RichText::new(text).font(egui::FontId::new(15.0, egui::FontFamily::Name("bold".into())))).selectable(false));
    ui.add_space(4.0);
}

impl App {
    pub fn set_key(&mut self, name: &'static str, a: String) {
        match name {
            "quickAdd" => self.cfg.quick_add = a,
            "find" => self.cfg.find = a,
            "add" => self.cfg.add = a,
            "hideDone" => self.cfg.hide_done = a,
            _ => self.cfg.help = a,
        }
        self.save_cfg();
    }

    fn mcp_json(&self) -> String {
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "notix".into());
        let v = serde_json::json!({"mcpServers": {"notix": {"command": exe, "args": ["--mcp"], "env": {"NOTIX_DIR": self.store.dir().display().to_string()}}}});
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.settings.open {
        return;
    }
    let p = app.pal;
    let tr = app.i18n.clone();
    let t = |k: &str| tr.t(k, &[]);
    let mut cfg = app.cfg.clone();
    let (mut cap, mut dir_note, copied) = (app.settings.cap, app.settings.dir_note, app.settings.mcp_copied);
    let (mut close, mut again, mut pick, mut copy_mcp) = (false, false, false, false);
    let sys = theme::dark_system(ctx);
    let max_h = (ctx.content_rect().height() - 90.0).clamp(200.0, 640.0);
    let frame = egui::Frame::new().fill(p.panel).stroke(Stroke::new(1.0, p.rule)).corner_radius(10.0).inner_margin(Margin::symmetric(20, 18));
    let m = egui::Modal::new(egui::Id::unique("settings")).frame(frame).show(ctx, |ui| {
        ui.set_width(440.0);
        egui::ScrollArea::vertical().min_scrolled_height(max_h - 48.0).max_height(max_h - 48.0).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            h3(ui, &p, &t("look"), true);
            ui.horizontal(|ui| {
                ui.label(t("language"));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let name = |v: &str| if v == "auto" { t("auto") } else { i18n::NAMES.iter().find(|(c, _)| *c == v).map_or(v.to_string(), |(_, n)| n.to_string()) };
                    egui::ComboBox::from_id_salt("set-lang").selected_text(name(&cfg.lang)).show_ui(ui, |ui| {
                        for v in LANGS {
                            ui.selectable_value(&mut cfg.lang, v.to_string(), name(v));
                        }
                    });
                });
            });
            ui.horizontal(|ui| {
                ui.label(t("theme"));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("set-theme").selected_text(t(&format!("th_{}", cfg.theme))).show_ui(ui, |ui| {
                        for v in THEMES {
                            ui.selectable_value(&mut cfg.theme, v.to_string(), t(&format!("th_{v}")));
                        }
                    });
                });
            });
            ui.horizontal(|ui| {
                ui.label(t("accent"));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    for v in ACCENTS.iter().rev() {
                        let (rect, r) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                        let on = cfg.accent == *v;
                        let col = theme::palette(&cfg.theme, v, sys).patina;
                        ui.painter().circle_filled(rect.center(), 11.0, col);
                        if on {
                            ui.painter().circle_stroke(rect.center(), 14.0, Stroke::new(2.0, p.vellum));
                        }
                        if r.has_focus() {
                            ui.painter().circle_stroke(rect.center(), 16.0, Stroke::new(2.0, p.patina));
                        }
                        let label = t(&format!("ac_{v}"));
                        r.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, on, &label));
                        if r.on_hover_text(label).clicked() {
                            cfg.accent = v.to_string();
                        }
                    }
                });
            });

            h3(ui, &p, &t("shortcuts"), false);
            for (name, label) in KEYS {
                ui.horizontal(|ui| {
                    // once sagdaki rozet + dugme, etiket kalan yere sarar (uzun kisayolun altina girmesin)
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let capturing = cap == Some(name);
                        if w::link(ui, &p, &t(if capturing { "cancelLow" } else { "change" }), LK::Plain).clicked() {
                            cap = if capturing { None } else { Some(name) };
                        }
                        let cur = match name {
                            "quickAdd" => &cfg.quick_add,
                            "find" => &cfg.find,
                            "add" => &cfg.add,
                            "hideDone" => &cfg.hide_done,
                            _ => &cfg.help,
                        };
                        let txt = if capturing { t("pressKeys") } else if cur.is_empty() { "—".into() } else { cur.clone() };
                        let c = if capturing { p.patina } else { p.dim };
                        egui::Frame::new().fill(p.ink).stroke(Stroke::new(1.0, if capturing { p.patina } else { p.rule })).corner_radius(4.0).inner_margin(Margin::symmetric(6, 2)).show(ui, |ui| {
                            ui.add(egui::Label::new(mono(txt, 11.0, c)).selectable(false));
                        });
                        ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                            ui.add(egui::Label::new(t(label)).wrap());
                        });
                    });
                });
            }
            ui.label(RichText::new(t("keysHint")).size(13.0).color(p.faint));

            h3(ui, &p, &t("app"), false);
            for (label, v) in [("oOpenAtLogin", &mut cfg.open_at_login), ("oAutoUpdate", &mut cfg.auto_update), ("oBackup", &mut cfg.backup), ("oNotify", &mut cfg.notify)] {
                ui.checkbox(v, t(label));
            }
            ui.horizontal(|ui| {
                ui.label(t("folder"));
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if !cfg.dir.is_empty() && w::link(ui, &p, &t("defaultDir"), LK::Plain).clicked() {
                        cfg.dir.clear();
                        dir_note = true;
                    }
                    if w::link(ui, &p, &t("changeDir"), LK::Plain).clicked() {
                        pick = true;
                    }
                });
            });
            ui.add(egui::Label::new(mono(app.store.dir().display().to_string(), 11.5, p.dim)));
            if dir_note {
                ui.label(RichText::new(t("restartNote")).size(13.0).color(p.faint));
            }
            ui.label(RichText::new(tr.t("version", &[("v", env!("CARGO_PKG_VERSION"))])).size(13.0).color(p.faint));

            h3(ui, &p, &t("ai"), false);
            ui.label(RichText::new(t("mcpHint")).size(13.0).color(p.faint));
            if ui.button(t(if copied { "mcpCopied" } else { "copyMcp" })).clicked() {
                copy_mcp = true;
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if w::link(ui, &p, &t("guideAgain"), LK::Plain).clicked() {
                again = true;
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.button(t("close")).clicked() {
                    close = true;
                }
            });
        });
    });
    if m.should_close() && cap.is_none() {
        close = true;
    }
    app.settings.cap = cap;
    app.settings.dir_note = dir_note;
    if copy_mcp {
        ctx.copy_text(app.mcp_json());
        app.settings.mcp_copied = true;
    }
    if pick {
        if let Some(d) = Desktop::pick_folder() {
            cfg.dir = d.display().to_string();
            app.settings.dir_note = true;
        }
    }
    if cfg != app.cfg {
        app.cfg = cfg;
        app.save_cfg();
    }
    if again {
        app.settings.open = false;
        guide::restart(app);
    } else if close {
        app.settings.open = false;
        app.settings.mcp_copied = false;
    }
}
