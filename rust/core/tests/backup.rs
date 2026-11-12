mod common;
use common::tmp;
use notix_core::{Kind, Store};
use std::process::Command;
use std::time::{Duration, Instant};

fn git_log(dir: &std::path::Path) -> Option<String> {
    let o = Command::new("git").arg("-C").arg(dir).args(["log", "--oneline"]).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

#[test]
fn yazimlar_tek_commit_olur_ve_kapaliyken_olmaz() {
    if Command::new("git").arg("--version").output().is_err() {
        return; // git yoksa uygulama zaten sessizce atlar
    }
    let s = Store::with_backup_delay(tmp(), Duration::from_millis(300));
    let slug = s.create("Yedekli").unwrap();
    s.add_task(&slug, "a", Kind::Task).unwrap();
    s.add_task(&slug, "b", Kind::Task).unwrap();
    let t = Instant::now();
    while git_log(s.dir()).is_none_or(|l| l.is_empty()) && t.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(git_log(s.dir()).unwrap().lines().count(), 1); // ardisik yazimlar tek commit

    s.set_backup(false);
    s.add_task(&slug, "c", Kind::Task).unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    assert_eq!(git_log(s.dir()).unwrap().lines().count(), 1);
}
