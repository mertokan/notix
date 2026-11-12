// Tema + vurgu + fontlar. Renkler public/style.css'ten birebir; color-mix(in srgb) = dogrusal karisim.
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, FontTweak, TextStyle};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub struct Pal {
    pub ink: Color32,
    pub panel: Color32,
    pub raise: Color32,
    pub rule: Color32,
    pub hover: Color32,
    pub vellum: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub amber: Color32,
    pub danger: Color32,
    pub danger_bg: Color32,
    pub danger_line: Color32,
    pub due_line: Color32,
    pub patina: Color32,
    pub sel: Color32,
    pub hit: Color32,
    pub light: bool,
}

fn hex(h: u32) -> Color32 {
    Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8)
}

/// a'yi b ile p (0..1) oraninda karistirir (opak renkler).
pub fn mix(a: Color32, b: Color32, p: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * (1.0 - p) + y as f32 * p).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0).round() as u8)
}

pub fn accent_color(name: &str) -> Color32 {
    hex(match name {
        "blue" => 0x6ea8e6,
        "violet" => 0xa99be8,
        "pink" => 0xe58bbd,
        "red" => 0xe88a80,
        "orange" => 0xe59a5b,
        "gold" => 0xd8b85a,
        "green" => 0x7cc47a,
        _ => 0x63b0a0,
    })
}

/// theme: dark|light|system|midnight|paper|forest; sys_dark: isletim sistemi koyu mu (system icin).
pub fn palette(theme: &str, accent: &str, sys_dark: bool) -> Pal {
    let theme = if theme == "system" { if sys_dark { "dark" } else { "light" } } else { theme };
    // dark taban; digerleri style.css'teki gibi uzerine yazar
    let mut p = Pal {
        ink: hex(0x191713), panel: hex(0x201d18), raise: hex(0x262219), rule: hex(0x2e2a23), hover: hex(0x3a352c),
        vellum: hex(0xece6da), dim: hex(0x948b7c), faint: hex(0x79705f), amber: hex(0xd9a441),
        danger: hex(0xd98070), danger_bg: hex(0x3a241f), danger_line: hex(0x4a2f29), due_line: hex(0x35544e),
        patina: Color32::BLACK, sel: Color32::BLACK, hit: Color32::BLACK, light: false,
    };
    let mut darken = 0.0;
    match theme {
        "midnight" => {
            (p.ink, p.panel, p.raise, p.rule, p.hover) = (hex(0x11141c), hex(0x171b26), hex(0x1d2230), hex(0x252b3a), hex(0x2f3648));
            (p.vellum, p.dim, p.faint) = (hex(0xe3e7f0), hex(0x8d96aa), hex(0x737d93));
            (p.danger, p.danger_bg, p.danger_line) = (hex(0xe08577), hex(0x3a2228), hex(0x4d2c33));
        }
        "forest" => {
            (p.ink, p.panel, p.raise, p.rule, p.hover) = (hex(0x121813), hex(0x182019), hex(0x1e281f), hex(0x263227), hex(0x314033));
            (p.vellum, p.dim, p.faint) = (hex(0xe4ebe1), hex(0x8fa08f), hex(0x748574));
            (p.danger, p.danger_bg, p.danger_line) = (hex(0xe08577), hex(0x3a2420), hex(0x4d2f29));
        }
        "light" => {
            (p.ink, p.panel, p.raise, p.rule, p.hover) = (hex(0xf7f6f3), hex(0xefede8), hex(0xe5e2db), hex(0xd9d5cc), hex(0xcfcac0));
            (p.vellum, p.dim, p.faint, p.amber) = (hex(0x1d1b17), hex(0x57524a), hex(0x6b665c), hex(0xa87200));
            (p.danger, p.danger_bg, p.danger_line) = (hex(0xb23b2a), hex(0xf6e1dc), hex(0xe6b9ae));
            (darken, p.light) = (0.42, true);
        }
        "paper" => {
            (p.ink, p.panel, p.raise, p.rule, p.hover) = (hex(0xf6f0e1), hex(0xefe7d4), hex(0xe6dcc5), hex(0xdbd0b6), hex(0xd0c4a8));
            (p.vellum, p.dim, p.faint, p.amber) = (hex(0x2b2418), hex(0x5b503e), hex(0x6c614d), hex(0x9c6b00));
            (p.danger, p.danger_bg, p.danger_line) = (hex(0xa8392a), hex(0xf3dcd2), hex(0xe0b4a6));
            (darken, p.light) = (0.42, true);
        }
        _ => {}
    }
    p.patina = mix(accent_color(accent), Color32::BLACK, darken);
    p.sel = alpha(p.patina, 0.28);
    p.hit = alpha(p.patina, 0.10);
    // koyu + patina birebir eski renk; gerisi vurgudan turer
    if accent != "patina" || theme != "dark" {
        p.due_line = mix(p.patina, p.ink, 0.6);
    }
    p
}

pub fn apply(ctx: &egui::Context, p: &Pal) {
    let mut v = if p.light { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.panel_fill = p.ink;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.ink;
    v.faint_bg_color = p.panel;
    v.override_text_color = Some(p.vellum);
    v.hyperlink_color = p.patina;
    v.window_stroke = egui::Stroke::new(1.0, p.rule);
    v.selection.bg_fill = p.sel;
    v.selection.stroke = egui::Stroke::new(1.0, p.patina);
    v.text_cursor.stroke = egui::Stroke::new(2.0, p.patina);
    v.window_corner_radius = 10.into();
    for (w, bg) in [
        (&mut v.widgets.noninteractive, p.panel),
        (&mut v.widgets.inactive, p.rule),
        (&mut v.widgets.hovered, p.hover),
        (&mut v.widgets.active, p.hover),
        (&mut v.widgets.open, p.hover),
    ] {
        w.bg_fill = bg;
        w.weak_bg_fill = bg;
        w.corner_radius = 6.into();
        w.fg_stroke.color = p.vellum;
    }
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.rule);
    v.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    v.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.patina);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        let ts = &mut s.text_styles;
        ts.insert(TextStyle::Body, FontId::proportional(15.0));
        ts.insert(TextStyle::Button, FontId::proportional(14.0));
        ts.insert(TextStyle::Small, FontId::proportional(11.5));
        ts.insert(TextStyle::Monospace, FontId::monospace(13.0));
        ts.insert(TextStyle::Heading, FontId::proportional(21.0));
        s.spacing.item_spacing = egui::vec2(8.0, 4.0);
        s.spacing.button_padding = egui::vec2(11.0, 6.0);
    });
}

pub fn dark_system(ctx: &egui::Context) -> bool {
    ctx.system_theme() != Some(egui::Theme::Light)
}

// Karisik satirda (ja + zh Han glifleri) farkli fontlarin taban cizgisi/boyu kaymasin.
fn tweak(name: &str, d: FontData) -> FontData {
    let (scale, y) = match name {
        "ja" => (1.0, 0.26),
        "zh" => (1.0, -0.1),
        _ => return d,
    };
    d.tweak(FontTweak { scale, y_offset_factor: y, ..Default::default() })
}

/// Sistem fontlari bir kez okunur; dil degisince sadece fallback sirasi degisir.
pub struct Fonts(Vec<(&'static str, Arc<FontData>)>);

impl Fonts {
    pub fn new() -> Fonts {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:/Windows".into());
        let files: [(&'static str, &[&str]); 6] = [
            ("sans", &["segoeui.ttf"]),
            ("sansb", &["segoeuib.ttf"]),
            ("sansi", &["segoeuii.ttf"]),
            ("mono", &["CascadiaMono.ttf", "consola.ttf"]),
            ("ja", &["YuGothM.ttc", "YuGothR.ttc", "meiryo.ttc"]),
            ("zh", &["msyh.ttc", "msyh.ttf", "simsun.ttc"]),
        ];
        // ilk bulunan dosya; hicbiri yoksa o font atlanir (egui varsayilani kalir)
        let list = files
            .iter()
            .filter_map(|(n, fs)| fs.iter().find_map(|f| std::fs::read(format!("{dir}/Fonts/{f}")).ok()).map(|b| (*n, Arc::new(tweak(n, FontData::from_owned(b))))))
            .collect();
        Fonts(list)
    }

    /// Han glifleri icin dile gore fallback sirasi: zh ise zh fontu ja'dan once.
    pub fn install(&self, ctx: &egui::Context, lang: &str) {
        let mut fd = FontDefinitions::default();
        for (n, d) in &self.0 {
            fd.font_data.insert((*n).into(), d.clone());
        }
        let has = |n: &str| self.0.iter().any(|(k, _)| *k == n);
        let cjk = if lang == "zh" { ["zh", "ja"] } else { ["ja", "zh"] };
        let fam = |first: &[&str]| -> Vec<String> {
            first.iter().chain(cjk.iter()).filter(|n| has(n)).map(|n| n.to_string()).collect()
        };
        // egui'nin varsayilan fontlari sonda kalir: sistem fontu yoksa yine de metin cizilir
        let (defp, defm) = (fd.families[&FontFamily::Proportional].clone(), fd.families[&FontFamily::Monospace].clone());
        let mut put = |f: FontFamily, mut list: Vec<String>, base: &[String]| {
            list.extend(base.iter().cloned());
            fd.families.insert(f, list);
        };
        put(FontFamily::Proportional, fam(&["sans"]), &defp);
        put(FontFamily::Monospace, fam(&["mono"]), &defm);
        put(FontFamily::Name("bold".into()), fam(&["sansb", "sans"]), &defp);
        put(FontFamily::Name("italic".into()), fam(&["sansi", "sans"]), &defp);
        ctx.set_fonts(fd);
    }
}
