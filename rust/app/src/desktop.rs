// Masaustu entegrasyonu: tepsi, global kisayol, bildirim, tek ornek, dosya izleme,
// acilista baslat, klasor secme. Arayuz bunlari sadece Event ve asagidaki
// fonksiyonlar uzerinden gorur.
// SOZLESME (patron yazdi): imzalari degistirme; degismesi gerekirse patrona yaz.
use crate::config::Config;
use eframe::egui::Context;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use notify::{RecursiveMode, Watcher};
use notix_core::Store;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// tepsi tiklamasi / "Goster" / ikinci acilis denemesi
    ShowWindow,
    /// bildirime tiklandi: pencereyi goster ve Bugun ekranini ac
    ShowToday,
    /// global hizli ekleme kisayolu ya da tepsi menusu
    QuickAdd,
    /// tepsi menusu "Cik"
    Quit,
    /// not klasorunde <slug>.md degisti (Notix'in kendisi ya da disaridan AI/editor)
    FileChanged(String),
}

pub struct Desktop {
    /// Olay gelince ctx.request_repaint() ile arayuz uyandirilir; arayuz logic()'te bosaltir.
    pub rx: Receiver<Event>,
    // bildirim tiklamasi icin (sadece Linux'ta okunur)
    #[allow(dead_code)]
    tx: Sender<Event>,
    #[allow(dead_code)]
    ctx: Context,
    tray: Option<TrayIcon>,
    hotkeys: Option<GlobalHotKeyManager>,
    bound: Option<HotKey>,
    /// tutulmazsa izleme durur
    _watcher: Option<notify::RecommendedWatcher>,
    notified_day: String,
}

// ikinci ornek sinyali: 127.0.0.1'de rastgele port, adresi NOTIX_HOME/instance.port'ta.
// Named pipe/unix soketi platforma ozel; TCP her yerde ayni kodla calisiyor.
static LISTENER: OnceLock<TcpListener> = OnceLock::new();

fn port_file() -> PathBuf {
    Config::home().join("instance.port")
}

/// Uygulama aciksa ona "show" gonder (ikinci ornek).
fn signal_existing() {
    let Some(port) = std::fs::read_to_string(port_file()).ok().and_then(|s| s.trim().parse::<u16>().ok()) else { return };
    if let Ok(mut s) = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(1)) {
        let _ = s.write_all(b"show");
    }
}

fn menu_labels(lang: &str) -> [&'static str; 3] {
    match lang {
        "tr" => ["Notix", "Hızlı ekle", "Çık"],
        "de" => ["Notix", "Schnell hinzufügen", "Beenden"],
        "es" => ["Notix", "Añadir rápido", "Salir"],
        "fr" => ["Notix", "Ajout rapide", "Quitter"],
        "pt" => ["Notix", "Adicionar rápido", "Sair"],
        "ru" => ["Notix", "Быстро добавить", "Выход"],
        "ja" => ["Notix", "クイック追加", "終了"],
        "zh" => ["Notix", "快速添加", "退出"],
        _ => ["Notix", "Quick add", "Quit"],
    }
}

fn due_text(lang: &str) -> &'static str {
    match lang {
        "tr" => "iş bugün ya da gecikmiş",
        "de" => "Aufgaben heute fällig oder überfällig",
        "es" => "tareas para hoy o atrasadas",
        "fr" => "tâches dues aujourd'hui ou en retard",
        "pt" => "tarefas para hoje ou atrasadas",
        "ru" => "задач на сегодня или просрочено",
        "ja" => "件のタスクが今日期限または期限切れ",
        "zh" => "项任务今天到期或已逾期",
        _ => "tasks due today or overdue",
    }
}

fn build_menu(lang: &str) -> Menu {
    let l = menu_labels(lang);
    let menu = Menu::new();
    let _ = menu.append_items(&[
        &MenuItem::with_id("show", l[0], true, None),
        &MenuItem::with_id("quick", l[1], true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("quit", l[2], true, None),
    ]);
    menu
}

/// Electron accelerator metni ("Control+Alt+N", "Super+K") -> HotKey. Tanimsiz parca = None.
fn parse_accel(s: &str) -> Option<HotKey> {
    let mut mods = Modifiers::empty();
    let mut key = None;
    for p in s.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match p.to_lowercase().as_str() {
            "control" | "ctrl" => mods |= Modifiers::CONTROL,
            "commandorcontrol" | "cmdorctrl" => {
                mods |= if cfg!(target_os = "macos") { Modifiers::META } else { Modifiers::CONTROL }
            }
            "alt" | "option" | "altgr" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "meta" | "command" | "cmd" | "win" => mods |= Modifiers::META,
            k => {
                if key.replace(parse_key(k)?).is_some() {
                    return None; // iki tus
                }
            }
        }
    }
    key.map(|k| HotKey::new(Some(mods).filter(|m| !m.is_empty()), k))
}

fn parse_key(k: &str) -> Option<Code> {
    use Code::*;
    const LETTERS: [Code; 26] = [KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ];
    const DIGITS: [Code; 10] = [Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9];
    const FN: [Code; 24] = [F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24];
    let b = k.as_bytes();
    if b.len() == 1 {
        match b[0] {
            c @ b'a'..=b'z' => return Some(LETTERS[(c - b'a') as usize]),
            c @ b'0'..=b'9' => return Some(DIGITS[(c - b'0') as usize]),
            _ => {}
        }
    }
    if let Some(n) = k.strip_prefix('f').and_then(|n| n.parse::<usize>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(FN[n - 1]);
    }
    Some(match k {
        "space" => Space,
        "enter" | "return" => Enter,
        "tab" => Tab,
        "escape" | "esc" => Escape,
        "backspace" => Backspace,
        "delete" => Delete,
        "insert" => Insert,
        "home" => Home,
        "end" => End,
        "pageup" => PageUp,
        "pagedown" => PageDown,
        "up" => ArrowUp,
        "down" => ArrowDown,
        "left" => ArrowLeft,
        "right" => ArrowRight,
        "," => Comma,
        "." => Period,
        "/" => Slash,
        ";" => Semicolon,
        "'" => Quote,
        "[" => BracketLeft,
        "]" => BracketRight,
        "\\" => Backslash,
        "-" => Minus,
        "=" => Equal,
        "`" => Backquote,
        "plus" => Equal, // "Plus" = Shift+= tusu; en yakin fiziksel tus
        _ => return None,
    })
}

/// Sadece `^[a-z0-9][a-z0-9-]*\.md$` (store.js'in proje adi kurali); baska dosya olay uretmez.
fn slug_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let slug = name.strip_suffix(".md")?;
    let mut ch = slug.chars();
    let first = ch.next()?;
    let ok = (first.is_ascii_lowercase() || first.is_ascii_digit()) && ch.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    ok.then(|| slug.to_string())
}

fn http_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    (u.starts_with("http://") || u.starts_with("https://")) && !url.chars().any(char::is_control)
}

/// Ayni dosyaya art arda gelen yazimlar tek olaya insin (~100ms).
fn debounce(raw: Receiver<String>, tx: Sender<Event>, ctx: Context) {
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        match raw.recv_timeout(Duration::from_millis(30)) {
            Ok(slug) => {
                pending.insert(slug, Instant::now());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
            Err(_) => {}
        }
        let due: Vec<String> = pending.iter().filter(|(_, t)| t.elapsed() >= Duration::from_millis(100)).map(|(s, _)| s.clone()).collect();
        for s in due {
            pending.remove(&s);
            send(&tx, &ctx, Event::FileChanged(s));
        }
    }
}

fn send(tx: &Sender<Event>, ctx: &Context, e: Event) {
    let _ = tx.send(e);
    ctx.request_repaint();
}

impl Desktop {
    /// main'de, eframe baslamadan once: false = baska ornek calisiyor (ona ShowWindow iletildi), cik.
    pub fn single_instance() -> bool {
        // Kilit process omru boyunca tutulmali; Drop'u bilerek calistirmiyoruz.
        let lock = match single_instance::SingleInstance::new("notix-rs") {
            Ok(l) => l,
            Err(_) => return true, // kilit kurulamadiysa acilisi engelleme
        };
        if !lock.is_single() {
            signal_existing();
            return false;
        }
        std::mem::forget(lock);
        if let Ok(l) = TcpListener::bind("127.0.0.1:0") {
            if let Ok(a) = l.local_addr() {
                let _ = std::fs::create_dir_all(Config::home());
                let _ = std::fs::write(port_file(), a.port().to_string());
            }
            let _ = LISTENER.set(l);
        }
        true
    }

    /// eframe creator icinde cagrilir (tepsi ve kisayol olay dongusu orada hazir).
    pub fn start(ctx: Context, cfg: &Config, notes: &Path) -> Desktop {
        let (tx, rx) = channel();

        // ikinci ornek sinyali (listener single_instance()'ta acildi)
        if let Some(l) = LISTENER.get().and_then(|l| l.try_clone().ok()) {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                for mut s in l.incoming().flatten() {
                    let mut buf = [0u8; 4];
                    let _ = s.set_read_timeout(Some(Duration::from_secs(1)));
                    if s.read_exact(&mut buf).is_ok() && &buf == b"show" {
                        send(&tx, &ctx, Event::ShowWindow);
                    }
                }
            });
        }

        // tepsi: menu ve sol tik olaylari ayri kanallardan gelir
        let tray = image::load_from_memory(include_bytes!("../assets/icon.png")).ok().and_then(|img| {
            let img = img.resize(32, 32, image::imageops::FilterType::Lanczos3).to_rgba8();
            let (w, h) = img.dimensions();
            let icon = tray_icon::Icon::from_rgba(img.into_raw(), w, h).ok()?;
            TrayIconBuilder::new().with_tooltip("Notix").with_icon(icon).with_menu_on_left_click(false).build().ok()
        });
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let ev = match e.id.0.as_str() {
                    "show" => Event::ShowWindow,
                    "quick" => Event::QuickAdd,
                    "quit" => Event::Quit,
                    _ => return,
                };
                send(&tx, &ctx, ev);
            }));
        }
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                    send(&tx, &ctx, Event::ShowWindow);
                }
            }));
        }
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                if e.state == HotKeyState::Pressed {
                    send(&tx, &ctx, Event::QuickAdd);
                }
            }));
        }

        // klasor izleme; baslamazsa 5sn poll yedek
        let watcher = {
            let (raw_tx, raw_rx) = channel::<String>();
            let (tx, ctx) = (tx.clone(), ctx.clone());
            std::thread::spawn(move || debounce(raw_rx, tx, ctx));
            notify::recommended_watcher(move |r: notify::Result<notify::Event>| {
                if let Ok(e) = r {
                    if !e.kind.is_access() {
                        e.paths.iter().filter_map(|p| slug_of(p)).for_each(|s| drop(raw_tx.send(s)));
                    }
                }
            })
            .ok()
            .and_then(|mut w| w.watch(notes, RecursiveMode::NonRecursive).ok().map(|_| w))
        };

        let mut d = Desktop {
            rx,
            tx,
            ctx,
            tray,
            hotkeys: GlobalHotKeyManager::new().ok(),
            bound: None,
            _watcher: watcher,
            notified_day: String::new(),
        };
        d.apply(cfg);
        d
    }

    /// Ayar degisince: kisayolu yeniden bagla, tepsi menusunu dile gore kur, acilista baslat.
    /// Donus: hizli ekleme kisayolu baglanabildi mi (false = baska uygulamada).
    pub fn apply(&mut self, cfg: &Config) -> bool {
        if let Some(t) = &self.tray {
            t.set_menu(Some(Box::new(build_menu(&cfg.lang()))));
        }
        let mgr = self.hotkeys.as_ref();
        if let (Some(m), Some(old)) = (mgr, self.bound.take()) {
            let _ = m.unregister(old);
        }
        let ok = match (mgr, parse_accel(&cfg.quick_add)) {
            (Some(m), Some(hk)) if m.register(hk).is_ok() => {
                self.bound = Some(hk);
                true
            }
            _ => false,
        };
        // Electron'daki app.isPackaged: gelistirme exe'si baslangic kaydina yazilmasin.
        if !cfg!(debug_assertions) {
            if let Some(al) = std::env::current_exe().ok().and_then(|p| {
                auto_launch::AutoLaunchBuilder::new().set_app_name("Notix").set_app_path(p.to_str()?).build().ok()
            }) {
                match (cfg.open_at_login, al.is_enabled().unwrap_or(false)) {
                    (true, false) => drop(al.enable()),
                    (false, true) => drop(al.disable()),
                    _ => {}
                }
            }
        }
        ok
    }

    /// Saatte bir cagrilir; notify acik ve bugun henuz bildirilmediyse tarihi gelen isler icin bildirim.
    pub fn notify_due(&mut self, store: &Store, cfg: &Config) {
        let day = notix_core::today();
        if !cfg.notify || self.notified_day == day {
            return;
        }
        let Ok(due) = store.due(&day) else { return };
        if due.is_empty() {
            return;
        }
        self.notified_day = day;
        let lang = cfg.lang();
        let body = std::iter::once(format!("{} {}", due.len(), due_text(&lang)))
            .chain(due.iter().take(3).map(|x| format!("• {}", x.text)))
            .collect::<Vec<_>>()
            .join("\n");
        let mut n = notify_rust::Notification::new();
        n.summary("Notix").body(&body);
        // shortcut: Windows'ta notify-rust tiklama olayi vermiyor (ShowToday gelmez); Linux'ta
        // "default" eylemi. macOS/Windows icin ayri bildirim kutuphanesi gerekir.
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            n.action("default", "Notix");
            if let Ok(h) = n.show() {
                let (tx, ctx) = (self.tx.clone(), self.ctx.clone());
                std::thread::spawn(move || h.wait_for_action(|a| if a == "default" { send(&tx, &ctx, Event::ShowToday) }));
            }
        }
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        let _ = n.show();
    }

    /// Sistem klasor secme penceresi (iptal = None).
    pub fn pick_folder() -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    /// Klasoru isletim sisteminin dosya gezgininde ac.
    pub fn open_folder(dir: &Path) {
        open_with_os(dir.as_os_str());
    }

    /// Baglantiyi varsayilan tarayicida ac (sadece http/https).
    pub fn open_url(url: &str) {
        if http_url(url) {
            open_with_os(url.as_ref());
        }
    }
}

fn open_with_os(target: &std::ffi::OsStr) {
    let (prog, pre): (&str, &[&str]) = if cfg!(windows) {
        ("explorer", &[]) // cmd /c start URL'deki & isaretini yorumlar; explorer dogrudan acar
    } else if cfg!(target_os = "macos") {
        ("open", &[])
    } else {
        ("xdg-open", &[])
    };
    let _ = Command::new(prog).args(pre).arg(target).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kisayol_ayristirici() {
        let k = |s| parse_accel(s);
        assert_eq!(k("Control+Alt+N"), Some(HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN)));
        assert_eq!(k("ctrl+shift+Space"), Some(HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)));
        assert_eq!(k("Super+K"), Some(HotKey::new(Some(Modifiers::META), Code::KeyK)));
        assert_eq!(k("Control+Alt+Shift+F9"), Some(HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::F9)));
        assert_eq!(k("Alt+Left").map(|h| h.key), Some(Code::ArrowLeft));
        assert_eq!(k("Control"), None);
        assert_eq!(k("Control+A+B"), None);
        assert_eq!(k("Control+Bilinmez"), None);
        assert_eq!(k("Control+F25"), None);
        assert_eq!(k(""), None);
    }

    #[test]
    fn slug_filtresi() {
        let s = |p: &str| slug_of(Path::new(p));
        assert_eq!(s("/n/is-1.md"), Some("is-1".into()));
        assert_eq!(s("/n/9a.md"), Some("9a".into()));
        assert_eq!(s("/n/README.md"), None);
        assert_eq!(s("/n/-x.md"), None);
        assert_eq!(s("/n/a_b.md"), None);
        assert_eq!(s("/n/a.md.tmp"), None);
        assert_eq!(s("/n/.trash"), None);
        assert_eq!(s("/n/a.txt"), None);
    }

    #[test]
    fn url_semasi() {
        assert!(http_url("https://example.com/a?b=1&c=2"));
        assert!(http_url("HTTP://example.com"));
        assert!(!http_url("javascript:alert(1)"));
        assert!(!http_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!http_url("C:\\Windows\\notepad.exe"));
        assert!(!http_url("https://a.com\n--flag"));
    }

    #[test]
    fn menu_dilleri() {
        assert_eq!(menu_labels("tr")[2], "Çık");
        assert_eq!(menu_labels("xx")[1], "Quick add");
    }
}
