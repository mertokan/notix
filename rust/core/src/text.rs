// Satir cozumleme: store.js'teki regex'lerin elle yazilmis karsiliklari (regex
// bagimliligi eklemek yerine). JS'in \s ve `.` davranisi birebir korunur.

// JS \s: Unicode bosluklari + BOM; Rust'in White_Space'inden farki U+0085 (JS'te yok).
pub fn is_ws(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{feff}'
}
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_ws)
}

// JS'te `.` satir sonu karakterlerini yemez; bu satirlar regex'e uymaz.
fn dot_ok(s: &str) -> bool {
    !s.contains(['\r', '\u{2028}', '\u{2029}'])
}

pub struct TaskLine<'a> {
    pub indent: &'a str,
    pub done: bool,
    pub text: &'a str,
}

// /^(\s*)- \[( |x|X)\] ?(.*)$/
pub fn task(line: &str) -> Option<TaskLine<'_>> {
    let body = line.trim_start_matches(is_ws);
    let indent = &line[..line.len() - body.len()];
    let mut it = body.strip_prefix("- [")?.chars();
    let mark = it.next().filter(|c| matches!(c, ' ' | 'x' | 'X'))?;
    let rest = it.as_str().strip_prefix(']')?;
    let text = rest.strip_prefix(' ').unwrap_or(rest);
    dot_ok(text).then_some(TaskLine { indent, done: mark != ' ', text })
}

// /^(#{1,6}) / — sadece onek (bolum sonu ve ad degistirme icin).
pub fn heading_level(line: &str) -> Option<usize> {
    let n = line.bytes().take_while(|&b| b == b'#').count();
    ((1..=6).contains(&n) && line[n..].starts_with(' ')).then_some(n)
}

// /^(#{1,6}) (.*)$/
pub fn heading(line: &str) -> Option<(usize, &str)> {
    let n = heading_level(line)?;
    let text = &line[n + 1..];
    dot_ok(text).then_some((n, text))
}

// Girinti = ic ice madde: bastaki bosluk/tab, tab iki sayilir.
pub fn indent_width(s: &str) -> usize {
    s.chars().take_while(|&c| c == ' ' || c == '\t').map(|c| if c == '\t' { 2 } else { 1 }).sum()
}

// /\r?\n/ ile bolme: tek basina \r satirin icinde kalir.
pub fn split_lines(s: &str) -> Vec<String> {
    let parts: Vec<&str> = s.split('\n').collect();
    let last = parts.len() - 1;
    parts
        .iter()
        .enumerate()
        .map(|(k, p)| if k < last { p.strip_suffix('\r').unwrap_or(p) } else { p }.to_string())
        .collect()
}

// Tarih satir icinde: ilk @YYYY-MM-DD.
pub fn date_in(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    (0..b.len().saturating_sub(10)).find_map(|i| {
        let w = &b[i..i + 11];
        let ok = w[0] == b'@'
            && w[1..].iter().enumerate().all(|(k, c)| if k == 4 || k == 7 { *c == b'-' } else { c.is_ascii_digit() });
        ok.then(|| &s[i + 1..i + 11])
    })
}

// "Ev İşleri" → ev-isleri (ASCII dosya adi, Turkce harfler kaybolmaz).
pub fn slugify(s: &str) -> String {
    let ascii: String = s
        .chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c,
        })
        .collect();
    let mut out = String::new();
    for c in ascii.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-');
    out.chars().take(60).collect()
}

// Node'un Buffer.from(s, 'base64') gibi hosgorulu: bosluk/gecersiz karakter atlanir,
// url-safe harfler kabul, '=' dosyayi bitirir.
pub fn base64_decode(s: &str) -> Vec<u8> {
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.chars() {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            '=' => break,
            _ => continue,
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_trkce_ve_kenarlar() {
        assert_eq!(slugify("Çöp Güncesi ıI"), "cop-guncesi-ii");
        assert_eq!(slugify("  --A__b--  "), "a-b");
        assert_eq!(slugify("../.."), "");
    }

    #[test]
    fn slug_60_karakterde_kesilir() {
        // JS: slice(0,60) sonrasi sondaki '-' kalabilir; burada da ayni
        let s = format!("{}-{}", "a".repeat(59), "b");
        assert_eq!(slugify(&s), format!("{}-", "a".repeat(59)));
    }

    #[test]
    fn gorev_satiri() {
        let t = task("  - [x] iş @2026-01-02").unwrap();
        assert_eq!((t.indent, t.done, t.text), ("  ", true, "iş @2026-01-02"));
        assert_eq!(task("- [ ]x").unwrap().text, "x");
        assert!(task("- [y] a").is_none() && task("-[ ] a").is_none() && task("- [ ] a\u{2028}b").is_none());
        assert_eq!(date_in("a @2026-09-15 b"), Some("2026-09-15"));
        assert_eq!(date_in("@2026-9-15"), None);
    }

    #[test]
    fn baslik_ve_satir_bolme() {
        assert_eq!(heading("## x"), Some((2, "x")));
        assert!(heading("####### x").is_none() && heading("#x").is_none());
        assert_eq!(split_lines("a\r\nb\rc\n"), ["a", "b\rc", ""]);
        assert_eq!(split_lines(""), [""]);
    }

    #[test]
    fn base64() {
        assert_eq!(base64_decode("ZGVuZW1l"), b"deneme");
        assert_eq!(base64_decode("AA=="), [0]);
        assert_eq!(base64_decode("ZGVu\nZW1l"), b"deneme");
    }
}
