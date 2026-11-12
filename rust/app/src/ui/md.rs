// Satir ici markdown: public/md.js ile ayni parcalar ve ayni guvenlik kurali
// (baglanti/resim sadece http(s) ya da media/<ad>.<uzanti>; digeri duz metin kalir).
use super::theme::Pal;
use eframe::egui::{self, text::LayoutJob, Color32, FontFamily, FontId, Stroke, TextFormat};

#[derive(Debug, Clone, PartialEq)]
pub enum Seg {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link(String, String),
    Img(String, String),
    Due(String),
}

pub fn safe(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    if l.starts_with("http://") || l.starts_with("https://") {
        return true;
    }
    let Some(rest) = l.strip_prefix("media/") else { return false };
    let Some((stem, ext)) = rest.split_once('.') else { return false };
    let mut c = stem.chars();
    c.next().is_some_and(|f| f.is_ascii_alphanumeric())
        && c.all(|x| x.is_ascii_alphanumeric() || x == '-')
        && !ext.is_empty()
        && ext.chars().all(|x| x.is_ascii_lowercase())
}

// `[etiket](adres)`: etiket ']' icermez, adres bosluk ve ')' icermez. s[i] = '['.
fn bracket(s: &str, i: usize, empty_ok: bool) -> Option<(&str, &str, usize)> {
    let close = i + 1 + s[i + 1..].find(']')?;
    let label = &s[i + 1..close];
    if label.is_empty() && !empty_ok {
        return None;
    }
    let rest = s[close + 1..].strip_prefix('(')?;
    let n = rest.find(|c: char| c == ')' || c.is_whitespace())?;
    if n == 0 || !rest[n..].starts_with(')') {
        return None;
    }
    Some((label, &rest[..n], close + 2 + n + 1))
}

// `sinir` ile cevrili, icinde `yasak` olmayan bos olmayan parca.
fn between<'a>(s: &'a str, i: usize, sinir: &str, yasak: char) -> Option<(&'a str, usize)> {
    let body = s[i + sinir.len()..].split(yasak).next()?;
    let end = i + sinir.len() + body.len();
    (!body.is_empty() && s[end..].starts_with(sinir)).then_some((body, end + sinir.len()))
}

fn at(s: &str, i: usize) -> Option<(Seg, usize)> {
    let tok = |end: usize| s[i..end].to_string();
    match s.as_bytes()[i] {
        b'!' if s[i + 1..].starts_with('[') => {
            let (alt, url, end) = bracket(s, i + 1, true)?;
            Some(if safe(url) { (Seg::Img(alt.into(), url.into()), end) } else { (Seg::Text(tok(end)), end) })
        }
        b'[' => {
            let (label, url, end) = bracket(s, i, false)?;
            Some(if safe(url) { (Seg::Link(label.into(), url.into()), end) } else { (Seg::Text(tok(end)), end) })
        }
        b'*' => between(s, i, "**", '*').map(|(b, e)| (Seg::Bold(b.into()), e)).or_else(|| between(s, i, "*", '*').map(|(b, e)| (Seg::Italic(b.into()), e))),
        b'`' => between(s, i, "`", '`').map(|(b, e)| (Seg::Code(b.into()), e)),
        b'@' => {
            let d = s.get(i + 1..i + 11)?;
            let ok = d.bytes().enumerate().all(|(k, b)| if k == 4 || k == 7 { b == b'-' } else { b.is_ascii_digit() });
            ok.then(|| (Seg::Due(d.into()), i + 11))
        }
        _ => None,
    }
}

pub fn parse(s: &str) -> Vec<Seg> {
    let mut out: Vec<Seg> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    while i < s.len() {
        match at(s, i) {
            Some((Seg::Text(t), end)) => {
                plain.push_str(&t);
                i = end;
            }
            Some((seg, end)) => {
                if !plain.is_empty() {
                    out.push(Seg::Text(std::mem::take(&mut plain)));
                }
                out.push(seg);
                i = end;
            }
            None => {
                let c = s[i..].chars().next().unwrap_or(' ');
                plain.push(c);
                i += c.len_utf8();
            }
        }
    }
    if !plain.is_empty() {
        out.push(Seg::Text(plain));
    }
    out
}

pub struct Style<'a> {
    pub pal: &'a Pal,
    pub size: f32,
    pub color: Color32,
    pub strike: bool,
    pub bold: bool,
    pub today: &'a str,
}

pub struct Rendered {
    pub job: LayoutJob,
    /// (baslangic, bitis) karakter indeksi + adres
    pub links: Vec<(usize, usize, String)>,
}

pub fn layout(segs: &[Seg], st: &Style) -> Rendered {
    let mut job = LayoutJob::default();
    let mut links = Vec::new();
    let mut off = 0;
    for seg in segs {
        let mut f = TextFormat { font_id: FontId::proportional(st.size), color: st.color, ..Default::default() };
        let s: String = match seg {
            Seg::Text(t) => t.clone(),
            Seg::Bold(t) => {
                f.font_id = FontId::new(st.size, FontFamily::Name("bold".into()));
                t.clone()
            }
            Seg::Italic(t) => {
                f.font_id = FontId::new(st.size, FontFamily::Name("italic".into()));
                t.clone()
            }
            Seg::Code(t) => {
                f.font_id = FontId::monospace(st.size - 2.0);
                f.background = st.pal.ink;
                format!("\u{2009}{t}\u{2009}")
            }
            Seg::Link(label, url) => {
                f.color = st.pal.patina;
                links.push((off, off + label.chars().count(), url.clone()));
                label.clone()
            }
            Seg::Due(d) => {
                f.font_id = FontId::monospace(st.size - 3.5);
                f.background = st.pal.panel;
                f.color = if st.strike {
                    st.pal.faint
                } else if d.as_str() < st.today {
                    st.pal.danger
                } else if d == st.today {
                    st.pal.patina
                } else {
                    st.pal.faint
                };
                format!("\u{2009}{d}\u{2009}")
            }
            Seg::Img(..) => continue, // resimler metnin altinda ayri cizilir
        };
        if st.bold && f.font_id.family == FontFamily::Proportional {
            f.font_id.family = FontFamily::Name("bold".into());
        }
        if st.strike && !matches!(seg, Seg::Due(_)) {
            f.strikethrough = Stroke::new(1.0, st.pal.faint);
        }
        off += s.chars().count();
        job.append(&s, 0.0, f);
    }
    Rendered { job, links }
}

/// Secilip kopyalanabilir zengin metin. Donus: (yanit, tiklanan baglanti adresi).
pub fn rich(ui: &mut egui::Ui, r: Rendered, width: f32) -> (egui::Response, Option<String>) {
    let mut job = r.job;
    job.wrap.max_width = width;
    let color = job.sections.first().map(|s| s.format.color).unwrap_or(Color32::GRAY);
    let (pos, galley, resp) = egui::Label::new(job).wrap_mode(egui::TextWrapMode::Wrap).sense(egui::Sense::click()).layout_in_ui(ui);
    if ui.is_rect_visible(resp.rect) {
        egui::text_selection::LabelSelectionState::label_text_selection(ui, &resp, pos, galley.clone(), color, Stroke::NONE);
    }
    let mut url = None;
    if let Some(p) = resp.hover_pos().or(resp.interact_pointer_pos()) {
        let i = galley.cursor_from_pos(p - pos).index.0;
        if let Some((_, _, u)) = r.links.iter().find(|(a, b, _)| i >= *a && i < *b) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                url = Some(u.clone());
            }
        }
    }
    (resp, url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parcalar() {
        let s = parse("a **kalin** *egik* `kod` [bag](https://x.y) ![r](media/a-1.png) @2026-09-15 son");
        assert_eq!(
            s,
            vec![
                Seg::Text("a ".into()),
                Seg::Bold("kalin".into()),
                Seg::Text(" ".into()),
                Seg::Italic("egik".into()),
                Seg::Text(" ".into()),
                Seg::Code("kod".into()),
                Seg::Text(" ".into()),
                Seg::Link("bag".into(), "https://x.y".into()),
                Seg::Text(" ".into()),
                Seg::Img("r".into(), "media/a-1.png".into()),
                Seg::Text(" ".into()),
                Seg::Due("2026-09-15".into()),
                Seg::Text(" son".into()),
            ]
        );
    }

    #[test]
    fn guvensiz_adres_duz_metin() {
        assert_eq!(parse("[x](javascript:alert(1))"), vec![Seg::Text("[x](javascript:alert(1))".into())]);
        assert_eq!(parse("![x](file:///c:/a.png)"), vec![Seg::Text("![x](file:///c:/a.png)".into())]);
        assert!(safe("HTTP://a.b") && safe("media/ab-1.PNG") && !safe("media/../x.png") && !safe("media/a.b.c"));
    }

    #[test]
    fn turkce_karakter_ve_bozuk_isaret() {
        assert_eq!(parse("çok ığ ** * ` [a]( @2026-1"), vec![Seg::Text("çok ığ ** * ` [a]( @2026-1".into())]);
    }
}
