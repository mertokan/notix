// Otomatik yedek: not klasoru kendi git deposu. Her yazimdan `delay` sonra tek
// commit (debounce); git yoksa sessizce atlanir. Ust depoya bulasmasin diye hep
// dir/.git kullanilir, yoksa once init edilir. Ayri thread: uygulama bloklanmaz.
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Once};
use std::time::{Duration, Instant};

struct State {
    on: bool,
    due: Option<Instant>,
    closed: bool,
}

pub struct Backup {
    dir: PathBuf,
    delay: Duration,
    state: Mutex<State>,
    wake: Condvar,
    started: Once,
}

impl Backup {
    pub fn new(dir: PathBuf, delay: Duration) -> Arc<Self> {
        Arc::new(Backup {
            dir,
            delay,
            state: Mutex::new(State { on: true, due: None, closed: false }),
            wake: Condvar::new(),
            started: Once::new(),
        })
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set(&self, on: bool) {
        let mut st = self.lock();
        st.on = on;
        if !on {
            st.due = None;
        }
    }

    pub fn close(&self) {
        self.lock().closed = true;
        self.wake.notify_all();
    }

    // Her yazimdan sonra cagrilir: sayaci sifirlar. Thread ilk yazimda acilir.
    pub fn touch(self: &Arc<Self>) {
        let mut st = self.lock();
        if !st.on {
            return;
        }
        st.due = Some(Instant::now() + self.delay);
        drop(st);
        self.wake.notify_all();
        self.started.call_once(|| {
            let me = Arc::clone(self);
            std::thread::spawn(move || me.worker());
        });
    }

    fn worker(&self) {
        let mut st = self.lock();
        loop {
            if st.closed {
                return;
            }
            match st.due {
                None => st = self.wake.wait(st).unwrap_or_else(|e| e.into_inner()),
                Some(t) if Instant::now() >= t => {
                    st.due = None;
                    drop(st);
                    self.commit();
                    st = self.lock();
                }
                Some(t) => {
                    st = self.wake.wait_timeout(st, t - Instant::now()).unwrap_or_else(|e| e.into_inner()).0;
                }
            }
        }
    }

    fn git(&self, args: &[&str]) -> bool {
        let mut c = Command::new("git");
        c.arg("-C").arg(&self.dir).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        std::os::windows::process::CommandExt::creation_flags(&mut c, 0x0800_0000); // pencere acma
        c.status().map(|s| s.success()).unwrap_or(false)
    }

    fn commit(&self) {
        if !self.dir.join(".git").exists() && !self.git(&["init", "-q"]) {
            return;
        }
        if self.git(&["add", "-A"]) {
            // Commit mesaji yerel saatle; degisiklik yoksa commit zaten basarisiz olur, sorun degil.
            let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
            self.git(&["-c", "user.name=Notix", "-c", "user.email=notix@local", "commit", "-q", "-m", &stamp]);
        }
    }
}
