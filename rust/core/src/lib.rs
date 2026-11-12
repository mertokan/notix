// Notix cekirdegi: store.js'in Rust karsiligi. Kaynak = <dir>/<slug>.md.
// Satir-tabanli: tanimadigimiz satirlara dokunmayiz, boylece AI'in/kullanicinin
// elle yazdigi her sey aynen korunur. Davranis ve hata metinleri store.js ile
// birebir ayni tutulur (arayuz metinleri anahtar olarak ceviriyor).
mod backup;
pub mod text;

use backup::Backup;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use text::{trim, TaskLine};

// --- hatalar: kod (400/404/409/413) + store.js'teki ayni Turkce ascii mesaj ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    BadRequest,
    NotFound,
    Conflict,
    TooLarge,
    /// Dosya yok (Node'daki ENOENT); HTTP'de 404, MCP'de ayri mesaj.
    Missing,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: Code,
    pub message: String,
}

impl Error {
    pub fn status(&self) -> u16 {
        match self.code {
            Code::BadRequest => 400,
            Code::NotFound | Code::Missing => 404,
            Code::Conflict => 409,
            Code::TooLarge => 413,
            Code::Io => 500,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        let code = if e.kind() == io::ErrorKind::NotFound { Code::Missing } else { Code::Io };
        Error { code, message: e.to_string() }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn bad(code: Code, msg: &str) -> Error {
    Error { code, message: msg.into() }
}

// --- veri tipleri ---
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Task { i: usize, done: bool, text: String, indent: usize },
    Heading { i: usize, level: usize, text: String },
    Text { i: usize, text: String, indent: usize },
}

impl Item {
    pub fn i(&self) -> usize {
        match self {
            Item::Task { i, .. } | Item::Heading { i, .. } | Item::Text { i, .. } => *i,
        }
    }
    pub fn text(&self) -> &str {
        match self {
            Item::Task { text, .. } | Item::Heading { text, .. } | Item::Text { text, .. } => text,
        }
    }
    pub fn json(&self) -> Value {
        match self {
            Item::Task { i, done, text, indent } => json!({"i": i, "kind": "task", "done": done, "text": text, "indent": indent}),
            Item::Heading { i, level, text } => json!({"i": i, "kind": "heading", "level": level, "text": text}),
            Item::Text { i, text, indent } => json!({"i": i, "kind": "text", "text": text, "indent": indent}),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub items: Vec<Item>,
}

impl Project {
    pub fn json(&self) -> Value {
        json!({"slug": self.slug, "title": self.title, "items": self.items.iter().map(Item::json).collect::<Vec<_>>()})
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub slug: String,
    pub title: String,
    pub open: usize,
    pub done: usize,
}

impl Summary {
    pub fn json(&self) -> Value {
        json!({"slug": self.slug, "title": self.title, "open": self.open, "done": self.done})
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    pub slug: String,
    pub project: String,
    pub i: usize,
    pub text: String,
    pub date: String,
}

impl Due {
    pub fn json(&self) -> Value {
        json!({"slug": self.slug, "project": self.project, "i": self.i, "text": self.text, "date": self.date})
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub slug: String,
    pub project: String,
    pub i: usize,
    pub kind: &'static str, // "task" | "text"
    pub done: Option<bool>,
    pub text: String,
}

impl Hit {
    pub fn json(&self) -> Value {
        let mut v = json!({"slug": self.slug, "project": self.project, "i": self.i, "kind": self.kind});
        if let Some(d) = self.done {
            v["done"] = d.into();
        }
        v["text"] = self.text.clone().into();
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub text: String,
    /// Baslik satiri; bolum yoksa -1.
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone)]
pub struct TrashEntry {
    pub file: String,
    pub title: String,
    /// mtime, ms (JS'teki mtimeMs).
    pub at: f64,
    pub open: usize,
    pub done: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Imported {
    pub slug: String,
    pub replaced: bool,
}

#[derive(Debug, Clone)]
pub struct MediaFile {
    pub path: PathBuf,
    pub content_type: &'static str,
}

/// add_task turu: 'task' (varsayilan), 'sub' (girintili alt is), 'note' (serbest metin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    Task,
    Sub,
    Note,
}

impl Kind {
    /// Bilinmeyen deger gorev sayilir (store.js'te de oyle).
    pub fn parse(s: Option<&str>) -> Kind {
        match s {
            Some("note") => Kind::Note,
            Some("sub") => Kind::Sub,
            _ => Kind::Task,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
}

#[derive(Debug, Default, Clone)]
pub struct TaskUpdate<'a> {
    pub expect: Option<&'a str>,
    pub done: Option<bool>,
    pub text: Option<&'a str>,
    pub remove: bool,
}

const MEDIA_TYPES: [(&str, &str); 6] = [
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
];

fn media_type(ext: &str) -> Option<&'static str> {
    MEDIA_TYPES.iter().find(|(e, _)| *e == ext).map(|(_, t)| *t)
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn join(lines: &[String]) -> String {
    lines.join("\n")
}

/// Yerel gun (YYYY-MM-DD); "bugun" istemcinin takvimine gore.
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn slugify(s: &str) -> String {
    text::slugify(s)
}

pub struct Store {
    dir: PathBuf,
    backup: Arc<Backup>,
}

impl Drop for Store {
    fn drop(&mut self) {
        self.backup.close();
    }
}

impl Store {
    /// Yedek gecikmesi NOTIX_BACKUP_MS (varsayilan 20000).
    pub fn new(dir: impl Into<PathBuf>) -> Store {
        let ms = std::env::var("NOTIX_BACKUP_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
        Store::with_backup_delay(dir, Duration::from_millis(ms))
    }

    pub fn with_backup_delay(dir: impl Into<PathBuf>, delay: Duration) -> Store {
        let dir = dir.into();
        Store { backup: Backup::new(dir.clone(), delay), dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn set_backup(&self, on: bool) {
        self.backup.set(on)
    }

    fn trash_dir(&self) -> PathBuf {
        self.dir.join(".trash")
    }
    fn media_dir(&self) -> PathBuf {
        self.dir.join("media")
    }

    fn file(&self, slug: &str) -> Result<PathBuf> {
        let mut c = slug.chars();
        let ok = c.next().is_some_and(|f| f.is_ascii_lowercase() || f.is_ascii_digit())
            && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-');
        if !ok {
            return Err(bad(Code::BadRequest, "gecersiz proje adi"));
        }
        Ok(self.dir.join(format!("{slug}.md")))
    }

    fn read_lines(&self, slug: &str) -> Result<Vec<String>> {
        Ok(text::split_lines(&self.read(slug)?))
    }

    fn write_lines(&self, slug: &str, lines: &[String]) -> Result<()> {
        fs::write(self.file(slug)?, join(lines))?;
        self.backup.touch();
        Ok(())
    }

    pub fn read(&self, slug: &str) -> Result<String> {
        Ok(String::from_utf8_lossy(&fs::read(self.file(slug)?)?).into_owned())
    }

    pub fn parse(&self, slug: &str) -> Result<Project> {
        let items: Vec<Item> = self
            .read_lines(slug)?
            .iter()
            .enumerate()
            .map(|(i, raw)| {
                if let Some(t) = text::task(raw) {
                    Item::Task { i, done: t.done, text: t.text.into(), indent: text::indent_width(t.indent) }
                } else if let Some((level, t)) = text::heading(raw) {
                    Item::Heading { i, level, text: t.into() }
                } else {
                    Item::Text { i, text: trim(raw).into(), indent: text::indent_width(raw) }
                }
            })
            .collect();
        let title = items
            .iter()
            .find_map(|x| if let Item::Heading { text, .. } = x { Some(text.clone()) } else { None })
            .unwrap_or_else(|| slug.to_string());
        Ok(Project { slug: slug.into(), title, items })
    }

    fn md_slugs(&self) -> Vec<String> {
        fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok()?.strip_suffix(".md").map(String::from))
            .collect()
    }

    pub fn list(&self) -> Result<Vec<Summary>> {
        fs::create_dir_all(&self.dir)?;
        // store.js bozuk adli bir .md'de (orn. README.md) butun listeyi 400'le dusuruyordu;
        // burada gecersiz adli dosya atlanir — uygulama acilsin.
        let mut out: Vec<Summary> = self
            .md_slugs()
            .iter()
            .filter_map(|s| self.parse(s).ok())
            .map(|p| {
                let tasks: Vec<bool> = p.items.iter().filter_map(|x| if let Item::Task { done, .. } = x { Some(*done) } else { None }).collect();
                let done = tasks.iter().filter(|d| **d).count();
                Summary { slug: p.slug, title: p.title, open: tasks.len() - done, done }
            })
            .collect();
        // localeCompare benzeri: harf buyuklugune bakmadan, esitse kucuk harf once.
        out.sort_by(|a, b| {
            b.open
                .cmp(&a.open)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                .then_with(|| b.title.cmp(&a.title))
        });
        Ok(out)
    }

    // --- cop kutusu: silinen projeler ve ice aktarmada uzerine yazilanlar burada ---
    fn trash_path(&self, name: &str) -> Result<PathBuf> {
        let stem = name.strip_suffix(".md");
        let ok = stem.is_some_and(|s| {
            let mut c = s.chars();
            c.next().is_some_and(|f| f.is_ascii_lowercase() || f.is_ascii_digit())
                && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-')
        });
        if !ok {
            return Err(bad(Code::BadRequest, "gecersiz dosya"));
        }
        Ok(self.trash_dir().join(name))
    }

    pub fn trash(&self) -> Result<Vec<TrashEntry>> {
        let Ok(rd) = fs::read_dir(self.trash_dir()) else { return Ok(vec![]) };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let Ok(file) = e.file_name().into_string() else { continue };
            if !file.ends_with(".md") {
                continue;
            }
            let raw = String::from_utf8_lossy(&fs::read(e.path())?).into_owned();
            let lines = text::split_lines(&raw);
            let tasks: Vec<bool> = lines.iter().filter_map(|l| text::task(l)).map(|t| t.done).collect();
            let done = tasks.iter().filter(|d| **d).count();
            let at = e.metadata()?.modified()?.duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
            let title = lines.iter().find_map(|l| text::heading(l)).map(|(_, t)| t.to_string()).unwrap_or_else(|| file.clone());
            out.push(TrashEntry { file, title, at, open: tasks.len() - done, done });
        }
        out.sort_by(|a, b| b.at.total_cmp(&a.at));
        Ok(out)
    }

    /// Ayni adda proje varsa uzerine yazmaz, bir sonraki bos ada geri alir.
    pub fn restore(&self, name: &str) -> Result<String> {
        let from = self.trash_path(name)?;
        let stem = name.strip_suffix(".md").unwrap_or(name);
        // "ad-<zaman>.md" → "ad"
        let digits = stem.bytes().rev().take_while(u8::is_ascii_digit).count();
        let base = match stem[..stem.len() - digits].strip_suffix('-') {
            Some(b) if digits > 0 => b,
            _ => stem,
        };
        if base.is_empty() {
            return Err(bad(Code::BadRequest, "gecersiz dosya"));
        }
        let mut slug = base.to_string();
        let mut n = 2;
        while self.file(&slug)?.exists() {
            slug = format!("{base}-{n}");
            n += 1;
        }
        fs::create_dir_all(&self.dir)?;
        fs::rename(from, self.file(&slug)?)?;
        self.backup.touch();
        Ok(slug)
    }

    /// Uygulamadaki tek gercek silme.
    pub fn purge(&self, name: &str) -> Result<()> {
        fs::remove_file(self.trash_path(name)?)?;
        self.backup.touch();
        Ok(())
    }

    pub fn create(&self, title: &str) -> Result<String> {
        let slug = slugify(title);
        if slug.is_empty() {
            return Err(bad(Code::BadRequest, "baslik bos"));
        }
        fs::create_dir_all(&self.dir)?;
        let f = self.file(&slug)?;
        if !f.exists() {
            fs::write(f, format!("# {}\n\n", trim(title)))?;
            self.backup.touch();
        }
        Ok(slug)
    }

    pub fn remove(&self, slug: &str) -> Result<()> {
        fs::create_dir_all(self.trash_dir())?;
        // silme yok, tasima
        fs::rename(self.file(slug)?, self.trash_dir().join(format!("{slug}-{}.md", now_ms())))?;
        self.backup.touch();
        Ok(())
    }

    pub fn add_task(&self, slug: &str, body: &str, kind: Kind) -> Result<()> {
        // Cok satirli yapistirma kopyalandigi gibi girsin: ilk satir secilen turu alir,
        // gerisi oldugu gibi yazilir (girinti, alt maddeler, aradaki bos satirlar durur).
        let mut yeni: Vec<String> = body.replace('\r', "").split('\n').map(|l| l.trim_end_matches(text::is_ws).to_string()).collect();
        while yeni.first().is_some_and(String::is_empty) {
            yeni.remove(0);
        }
        while yeni.last().is_some_and(String::is_empty) {
            yeni.pop();
        }
        if yeni.is_empty() {
            return Err(bad(Code::BadRequest, "bos gorev"));
        }
        let bas = trim(&yeni[0]).to_string();
        yeni[0] = match kind {
            Kind::Note => bas,
            Kind::Sub => format!("  - [ ] {bas}"),
            Kind::Task => format!("- [ ] {bas}"),
        };
        self.insert(slug, yeni)
    }

    /// Satirlari dosyaya koyar: bellek bolumu varsa onun ustune, yoksa sona.
    fn insert(&self, slug: &str, yeni: Vec<String>) -> Result<()> {
        let mut lines = self.read_lines(slug)?;
        if let Some((bas, _)) = mem_range(&lines) {
            let mut i = bas;
            while i > 0 && trim(&lines[i - 1]).is_empty() {
                i -= 1;
            }
            lines.splice(i..i, yeni);
        } else {
            while lines.last().is_some_and(|l| trim(l).is_empty()) {
                lines.pop();
            }
            lines.extend(yeni);
            lines.push(String::new());
        }
        self.write_lines(slug, &lines)
    }

    // Satirin yerini bulup `expect` ile dogrular (tasima islemlerinin ortak girisi).
    fn task_at<'a>(lines: &'a [String], i: usize, expect: Option<&str>) -> Result<TaskLine<'a>> {
        match lines.get(i).and_then(|l| text::task(l)) {
            Some(m) if expect.is_none_or(|e| m.text == e) => Ok(m),
            _ => Err(bad(Code::Conflict, "satir degismis")),
        }
    }

    /// Komsu gorev satiriyla yer degistirir; aradaki baslik/serbest metin yerinde kalir.
    // shortcut: alt maddeleri birlikte tasimaz (JS ile ayni), gerekirse blok tasimaya cevrilir.
    pub fn move_task(&self, slug: &str, i: usize, dir: Dir, expect: Option<&str>) -> Result<()> {
        let mut lines = self.read_lines(slug)?;
        Self::task_at(&lines, i, expect)?;
        let mut j = i;
        loop {
            j = match dir {
                Dir::Up if j == 0 => return Ok(()), // listenin ucu, yapacak bir sey yok
                Dir::Up => j - 1,
                Dir::Down => j + 1,
            };
            match lines.get(j) {
                None => return Ok(()),
                Some(l) if text::task(l).is_some() => break,
                _ => {}
            }
        }
        lines.swap(i, j);
        self.write_lines(slug, &lines)
    }

    /// Satiri baska projeye tasir: kaynaktan silinir, hedefe ust seviye is olarak girer.
    pub fn move_to_project(&self, slug: &str, i: usize, target: &str, expect: Option<&str>) -> Result<String> {
        if target == slug {
            return Err(bad(Code::BadRequest, "ayni proje"));
        }
        if !self.file(target)?.exists() {
            return Err(bad(Code::NotFound, "hedef proje yok"));
        }
        let mut lines = self.read_lines(slug)?;
        Self::task_at(&lines, i, expect)?;
        let satir = lines[i].trim_start_matches(text::is_ws).to_string();
        lines.remove(i);
        self.write_lines(slug, &lines)?;
        self.insert(target, vec![satir])?;
        Ok(target.to_string())
    }

    /// Tarihi gecmis/bugun olan acik isler — tum projelerden, "Bugun" gorunumu icin.
    pub fn due(&self, today: &str) -> Result<Vec<Due>> {
        let mut out = Vec::new();
        for Summary { slug, title, .. } in self.list()? {
            for x in self.parse(&slug)?.items {
                if let Item::Task { i, done: false, text, .. } = x {
                    if let Some(date) = text::date_in(&text).filter(|d| *d <= today) {
                        out.push(Due { slug: slug.clone(), project: title.clone(), i, date: date.into(), text });
                    }
                }
            }
        }
        out.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(out)
    }

    /// Basligi ve dosya adini degistirir; geri kalan satirlara dokunmaz.
    pub fn rename(&self, slug: &str, title: &str) -> Result<String> {
        let t = trim(&collapse_breaks(title)).to_string();
        let next = slugify(&t);
        if next.is_empty() {
            return Err(bad(Code::BadRequest, "baslik bos"));
        }
        if next != slug && self.file(&next)?.exists() {
            return Err(bad(Code::Conflict, "bu isimde proje var"));
        }
        let mut lines = self.read_lines(slug)?;
        match lines.iter().position(|l| text::heading_level(l).is_some()) {
            None => lines.insert(0, format!("# {t}")),
            Some(i) => lines[i] = format!("{} {t}", "#".repeat(text::heading_level(&lines[i]).unwrap_or(1))),
        }
        self.write_lines(slug, &lines)?;
        if next != slug {
            fs::rename(self.file(slug)?, self.file(&next)?)?;
            self.backup.touch();
        }
        Ok(next)
    }

    pub fn search(&self, q: &str) -> Result<Vec<Hit>> {
        let q = trim(q).to_lowercase();
        if q.encode_utf16().count() < 2 {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for Summary { slug, title, .. } in self.list()? {
            for x in self.parse(&slug)?.items {
                let (kind, done) = match &x {
                    Item::Heading { .. } => continue,
                    Item::Task { done, .. } => ("task", Some(*done)),
                    Item::Text { .. } => ("text", None),
                };
                if !trim(x.text()).is_empty() && x.text().to_lowercase().contains(&q) {
                    out.push(Hit { slug: slug.clone(), project: title.clone(), i: x.i(), kind, done, text: x.text().into() });
                }
            }
        }
        out.truncate(200);
        Ok(out)
    }

    pub fn export_all(&self) -> Result<BTreeMap<String, String>> {
        fs::create_dir_all(&self.dir)?;
        let mut out = BTreeMap::new();
        for s in self.md_slugs() {
            out.insert(s.clone(), String::from_utf8_lossy(&fs::read(self.dir.join(format!("{s}.md")))?).into_owned());
        }
        Ok(out)
    }

    /// Ustune yazmadan once eskisini .trash'e kopyalar — ice aktarma veri kaybettirmesin.
    pub fn import_file(&self, slug: &str, body: &str) -> Result<Imported> {
        let s = slugify(slug);
        if s.is_empty() {
            return Err(bad(Code::BadRequest, "gecersiz dosya adi"));
        }
        fs::create_dir_all(&self.dir)?;
        let f = self.file(&s)?;
        let replaced = f.exists();
        if replaced {
            fs::create_dir_all(self.trash_dir())?;
            fs::copy(&f, self.trash_dir().join(format!("{s}-{}.md", now_ms())))?;
        }
        fs::write(&f, body.replace("\r\n", "\n"))?;
        self.backup.touch();
        Ok(Imported { slug: s, replaced })
    }

    // --- proje bellegi: "## Bellek"/"## Memory" basligi altindaki satirlar. Ayri dosya
    // degil, cunku ajanlar zaten proje dosyasini okuyor. ---
    pub fn memory(&self, slug: &str) -> Result<Memory> {
        let lines = self.read_lines(slug)?;
        Ok(match mem_range(&lines) {
            None => Memory { text: String::new(), from: -1, to: -1 },
            Some((bas, son)) => Memory { text: lines[bas + 1..son].join("\n").trim_matches('\n').to_string(), from: bas as i64, to: son as i64 },
        })
    }

    /// expect = istemcinin gordugu bellek metni; arada AI yazdiysa 409.
    /// heading: bolum yoksa acilacak baslik ("Memory" ya da varsayilan "Bellek"); varsa dosyadaki korunur.
    pub fn set_memory(&self, slug: &str, body: &str, expect: Option<&str>, heading: Option<&str>) -> Result<()> {
        let simdi = self.memory(slug)?;
        if expect.is_some_and(|e| simdi.text != e) {
            return Err(bad(Code::Conflict, "bellek arada degismis"));
        }
        let mut lines = self.read_lines(slug)?;
        let govde = body.replace("\r\n", "\n");
        let govde = govde.trim_matches('\n');
        let govde_satirlari = || govde.split('\n').map(String::from);

        if simdi.from >= 0 {
            let (from, to) = (simdi.from as usize, simdi.to as usize);
            // bos birakildiysa bolumu tamamen kaldir
            let yeni: Vec<String> = if govde.is_empty() {
                vec![]
            } else {
                [lines[from].clone(), String::new()].into_iter().chain(govde_satirlari()).chain([String::new()]).collect()
            };
            lines.splice(from..to, yeni);
        } else if !govde.is_empty() {
            // Bolum dosyanin sonuna: "sonraki basliga kadar" kurali yuzunden ortada
            // olsa altindaki her seyi yutardi. Yeni isler de bunun ustune girer.
            while lines.last().is_some_and(|l| trim(l).is_empty()) {
                lines.pop();
            }
            let baslik = format!("## {}", if heading == Some("Memory") { "Memory" } else { "Bellek" });
            lines.extend([String::new(), baslik, String::new()]);
            lines.extend(govde_satirlari());
            lines.push(String::new());
        }
        self.write_lines(slug, &lines)
    }

    /// Belge modu: expect = duzenlemeye baslandigi andaki icerik; arada degistiyse 409.
    pub fn write(&self, slug: &str, body: &str, expect: Option<&str>) -> Result<()> {
        if let Some(e) = expect {
            if self.read(slug)? != e {
                return Err(bad(Code::Conflict, "dosya arada degismis"));
            }
        }
        fs::write(self.file(slug)?, body.replace("\r\n", "\n"))?;
        self.backup.touch();
        Ok(())
    }

    // --- resimler: <dir>/media/ ---
    pub fn save_media(&self, name: &str, base64: &str) -> Result<String> {
        let ext = name.rsplit('.').next().unwrap_or(name).to_lowercase();
        if media_type(&ext).is_none() {
            return Err(bad(Code::BadRequest, "sadece resim dosyasi"));
        }
        let buf = text::base64_decode(base64);
        if buf.is_empty() {
            return Err(bad(Code::BadRequest, "bos dosya"));
        }
        if buf.len() > 5_000_000 {
            return Err(bad(Code::TooLarge, "resim 5MB dan buyuk"));
        }
        fs::create_dir_all(self.media_dir())?;
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        let slug = slugify(stem);
        let f = format!("{}-{}.{ext}", if slug.is_empty() { "resim" } else { &slug }, now_ms());
        fs::write(self.media_dir().join(&f), buf)?;
        self.backup.touch();
        Ok(format!("media/{f}"))
    }

    /// Disaridan gelen ad: sadece bildigimiz uzantilar, klasor disina cikamaz.
    pub fn media_file(&self, name: &str) -> Result<MediaFile> {
        let found = name.split_once('.').and_then(|(stem, ext)| {
            let mut c = stem.chars();
            let stem_ok = c.next().is_some_and(|f| f.is_ascii_alphanumeric()) && c.all(|x| x.is_ascii_alphanumeric() || x == '-');
            let ext_ok = !ext.is_empty() && ext.chars().all(|x| x.is_ascii_alphabetic());
            (stem_ok && ext_ok).then(|| media_type(&ext.to_lowercase())).flatten()
        });
        match found {
            Some(content_type) => Ok(MediaFile { path: self.media_dir().join(name), content_type }),
            None => Err(bad(Code::BadRequest, "gecersiz dosya")),
        }
    }

    /// expect: istemcinin gordugu metin. Uymuyorsa dosya arada degismis (AI yazmis) → 409.
    pub fn update_task(&self, slug: &str, i: usize, u: TaskUpdate) -> Result<()> {
        let mut lines = self.read_lines(slug)?;
        let m = Self::task_at(&lines, i, u.expect)?;
        if u.remove {
            lines.remove(i);
        } else {
            let done = u.done.unwrap_or(m.done);
            let body = match u.text {
                Some(t) => trim(&collapse_breaks(t)).to_string(),
                None => m.text.to_string(),
            };
            lines[i] = format!("{}- [{}] {body}", m.indent, if done { 'x' } else { ' ' });
        }
        self.write_lines(slug, &lines)
    }
}

// /[\r\n]+/g → ' '
fn collapse_breaks(s: &str) -> String {
    let mut out = String::new();
    let mut prev_break = false;
    for c in s.chars() {
        let br = c == '\r' || c == '\n';
        if !br {
            out.push(c);
        } else if !prev_break {
            out.push(' ');
        }
        prev_break = br;
    }
    out
}

// Bellek bolumu: [baslik satiri, bitis). Ayni/ust seviye baska baslik baslayinca biter.
fn mem_range(lines: &[String]) -> Option<(usize, usize)> {
    let is_mem = |l: &str| -> Option<usize> {
        let n = l.bytes().take_while(|&b| b == b'#').count();
        let rest = l[n..].trim_matches(' ');
        ((2..=6).contains(&n) && (rest.eq_ignore_ascii_case("bellek") || rest.eq_ignore_ascii_case("memory"))).then_some(n)
    };
    let (bas, seviye) = lines.iter().enumerate().find_map(|(i, l)| is_mem(l).map(|n| (i, n)))?;
    let son = lines[bas + 1..].iter().position(|l| text::heading_level(l).is_some_and(|h| h <= seviye)).map_or(lines.len(), |p| bas + 1 + p);
    Some((bas, son))
}
