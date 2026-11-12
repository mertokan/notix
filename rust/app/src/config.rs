// Ayarlar: Electron surumunun .notix-config.json'u ile ayni alanlar ve dogrulama
// (server.js writeConfig). Dosya ayri tutulur (%APPDATA%/notix-rs/config.json):
// iki surum ayni anda acikken ayni global kisayolu kapmasin. Not klasoru ortak.
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

pub const LANGS: &[&str] = &["auto", "tr", "en", "de", "es", "fr", "pt", "ru", "ja", "zh"];
pub const THEMES: &[&str] = &["dark", "light", "system", "midnight", "paper", "forest"];
pub const ACCENTS: &[&str] = &["patina", "blue", "violet", "pink", "red", "orange", "gold", "green"];

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    // kisayollar: Electron accelerator metni ("Control+Alt+N")
    pub quick_add: String,
    pub find: String,
    pub add: String,
    pub hide_done: String,
    pub help: String,
    pub open_at_login: bool,
    pub auto_update: bool,
    pub backup: bool,
    pub notify: bool,
    pub lang: String,
    pub theme: String,
    pub accent: String,
    /// "" = varsayilan klasor
    pub dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            quick_add: "Control+Alt+N".into(),
            find: "Control+K".into(),
            add: "N".into(),
            hide_done: "H".into(),
            help: "?".into(),
            open_at_login: false,
            auto_update: true,
            backup: true,
            notify: true,
            lang: "auto".into(),
            theme: "dark".into(),
            accent: "patina".into(),
            dir: String::new(),
        }
    }
}

impl Config {
    /// Bilinmeyen/gecersiz alan varsayilana duser (server.js ile ayni kural).
    pub fn from_json(v: &Value) -> Config {
        let d = Config::default();
        let o = v.as_object().cloned().unwrap_or_default();
        let key = |k: &str, def: &str| match o.get(k).and_then(Value::as_str).map(str::trim) {
            Some(s) if !s.is_empty() && s.len() <= 40 => s.to_string(),
            _ => def.to_string(),
        };
        let flag = |k: &str, def: bool| o.get(k).and_then(Value::as_bool).unwrap_or(def);
        let pick = |k: &str, list: &[&str]| {
            o.get(k).and_then(Value::as_str).filter(|s| list.contains(s)).unwrap_or(list[0]).to_string()
        };
        let dir = o.get("dir").and_then(Value::as_str).unwrap_or("");
        let dir = if dir.is_empty() || (Path::new(dir).is_absolute() && Path::new(dir).is_dir()) { dir } else { "" };
        Config {
            quick_add: key("quickAdd", &d.quick_add),
            find: key("find", &d.find),
            add: key("add", &d.add),
            hide_done: key("hideDone", &d.hide_done),
            help: key("help", &d.help),
            open_at_login: flag("openAtLogin", d.open_at_login),
            auto_update: flag("autoUpdate", d.auto_update),
            backup: flag("backup", d.backup),
            notify: flag("notify", d.notify),
            lang: pick("lang", LANGS),
            theme: pick("theme", THEMES),
            accent: pick("accent", ACCENTS),
            dir: dir.to_string(),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "quickAdd": self.quick_add, "find": self.find, "add": self.add, "hideDone": self.hide_done, "help": self.help,
            "openAtLogin": self.open_at_login, "autoUpdate": self.auto_update, "backup": self.backup, "notify": self.notify,
            "lang": self.lang, "theme": self.theme, "accent": self.accent, "dir": self.dir,
        })
    }

    /// NOTIX_HOME (testler/gelistirme), yoksa %APPDATA%/notix-rs.
    pub fn home() -> PathBuf {
        match std::env::var_os("NOTIX_HOME").filter(|d| !d.is_empty()) {
            Some(d) => d.into(),
            None => dirs::config_dir().unwrap_or_default().join("notix-rs"),
        }
    }

    pub fn load() -> Config {
        let v = std::fs::read_to_string(Self::home().join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Object(Map::new()));
        Config::from_json(&v)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let home = Self::home();
        std::fs::create_dir_all(&home)?;
        std::fs::write(home.join("config.json"), serde_json::to_string_pretty(&self.to_json()).unwrap())
    }

    /// "auto" cozulmus dil kodu (sistem dili listede yoksa en). NOTIX_LANG testler icin.
    pub fn lang_code(&self, system: Option<&str>) -> String {
        if self.lang != "auto" {
            return self.lang.clone();
        }
        let sys = std::env::var("NOTIX_LANG").ok().or(system.map(String::from)).unwrap_or_default();
        let base = sys.split(['-', '_']).next().unwrap_or("").to_lowercase();
        if LANGS[1..].contains(&base.as_str()) { base } else { "en".into() }
    }
}

/// Not klasoru: NOTIX_DIR > ayarlardaki dir > Belgeler/Notix (kurulu Electron ile ayni).
pub fn notes_dir(cfg: &Config) -> PathBuf {
    if let Some(d) = std::env::var_os("NOTIX_DIR").filter(|d| !d.is_empty()) {
        return d.into();
    }
    if !cfg.dir.is_empty() {
        return cfg.dir.clone().into();
    }
    dirs::document_dir().unwrap_or_default().join("Notix")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gecersiz_alanlar_varsayilana_duser() {
        let c = Config::from_json(&json!({"lang": "xx", "theme": "light", "find": "", "notify": "evet", "dir": "goreli/yol", "extra": 1}));
        assert_eq!(c.lang, "auto");
        assert_eq!(c.theme, "light");
        assert_eq!(c.find, "Control+K");
        assert!(c.notify);
        assert_eq!(c.dir, "");
        assert_eq!(Config::from_json(&c.to_json()), c);
        assert_eq!(Config::default().lang_code(Some("ja-JP")), "ja");
        assert_eq!(Config::default().lang_code(Some("ko-KR")), "en");
    }
}
