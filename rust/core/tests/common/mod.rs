#![allow(dead_code)]
use notix_core::Store;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static N: AtomicUsize = AtomicUsize::new(0);

pub fn tmp() -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("notix-rs-{}-{}-{nanos}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&d).unwrap();
    d
}

// Yedek bu surede tetiklenmez (testler git'e dokunmasin).
pub fn store() -> Store {
    Store::with_backup_delay(tmp(), Duration::from_secs(3600))
}
