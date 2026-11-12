// Dil sozlugu: public/i18n.js'ten assets/convert-i18n.js ile uretilen JSON.
// Anahtar yoksa en, o da yoksa anahtarin kendisi (i18n.js ile ayni).
use serde_json::Value;

pub const NAMES: &[(&str, &str)] = &[
    ("tr", "Türkçe"), ("en", "English"), ("de", "Deutsch"), ("es", "Español"), ("fr", "Français"),
    ("pt", "Português"), ("ru", "Русский"), ("ja", "日本語"), ("zh", "中文"),
];

pub struct I18n {
    data: Value,
    pub lang: String,
}

/// Intl.PluralRules karsiligi (sadece tam sayilar; sozlukte olmayan kategori other'a duser).
pub fn plural(lang: &str, n: u64) -> &'static str {
    match lang {
        "ja" | "zh" => "other",
        "ru" => {
            let (m10, m100) = (n % 10, n % 100);
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "fr" | "pt" => if n <= 1 { "one" } else { "other" },
        _ => if n == 1 { "one" } else { "other" },
    }
}

impl I18n {
    pub fn new(lang: &str) -> I18n {
        let data = serde_json::from_str(include_str!("../../assets/i18n.json")).expect("i18n.json");
        I18n { data, lang: lang.into() }
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.data[&self.lang].get(key).or_else(|| self.data["en"].get(key))
    }

    /// `{ad}` yer tutucularini args ile doldurur; deger nesneyse ("n" argumanina gore) cogul secilir.
    pub fn t(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = match self.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Object(o)) => {
                let n = args.iter().find(|(k, _)| *k == "n").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
                o.get(plural(&self.lang, n)).or_else(|| o.get("other")).and_then(Value::as_str).unwrap_or(key).to_string()
            }
            _ => key.to_string(),
        };
        for (k, v) in args {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }

    /// Sunucu hata metni (sabit Turkce ascii) → secili dil; bilinmeyen oldugu gibi kalir.
    pub fn te(&self, e: &notix_core::Error) -> String {
        let err = |k: &str| self.data[&self.lang]["err"].get(k).and_then(Value::as_str).map(String::from);
        if e.code == notix_core::Code::Missing {
            return err("noProject").unwrap_or_else(|| e.message.clone());
        }
        err(&e.message).unwrap_or_else(|| e.message.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cogul_kategorileri() {
        for (n, k) in [(1, "one"), (2, "few"), (5, "many"), (11, "many"), (21, "one"), (22, "few"), (112, "many")] {
            assert_eq!(plural("ru", n), k, "ru {n}");
        }
        assert_eq!(plural("fr", 0), "one");
        assert_eq!(plural("en", 0), "other");
        assert_eq!(plural("ja", 1), "other");
    }

    #[test]
    fn tum_dillerde_tum_anahtarlar_var() {
        let d = I18n::new("tr").data;
        for (lang, _) in NAMES {
            for k in d["tr"].as_object().unwrap().keys() {
                assert!(d[lang].get(k).is_some(), "{lang}: {k} eksik");
            }
        }
    }

    #[test]
    fn t_ve_te() {
        let en = I18n::new("en");
        assert_eq!(en.t("qOpen", &[("n", "1")]), "1 open");
        let es = I18n::new("es");
        assert_eq!(es.t("qOpen", &[("n", "1")]), "1 abierta");
        assert_eq!(es.t("qOpen", &[("n", "3")]), "3 abiertas");
        assert_eq!(en.t("yokboyleanahtar", &[]), "yokboyleanahtar");
        let e = notix_core::Error { code: notix_core::Code::Conflict, message: "satir degismis".into() };
        assert_ne!(en.te(&e), "satir degismis");
        let tr = I18n::new("tr");
        assert_eq!(tr.te(&e), tr.data["tr"]["err"]["satir degismis"].as_str().unwrap_or("satir degismis"));
    }
}
