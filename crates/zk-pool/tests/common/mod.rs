//! A pool directory of its own for each test, removed afterwards.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use zk_pool::Receipt;

pub struct ScratchDir(PathBuf);

impl ScratchDir {
    pub fn new(label: &str) -> Self {
        // Tests run in parallel threads of one process and can read the same clock value, so a
        // counter keeps two directories from ever sharing a name and deleting each other's files.
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let name = format!("zk-pool-{label}-{}-{nanos}-{unique}", std::process::id());
        Self(std::env::temp_dir().join(name))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // Leftovers in the temporary directory are harmless if removal fails.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One line per shielded transaction with the numbers the report needs.
pub fn timing_line(system: &str, label: &str, receipt: &Receipt) -> String {
    let shielded = receipt.public.shielded.as_ref().unwrap();
    format!(
        "| {system} | {label} | {} | {:.1} | {:.1} | {} |",
        receipt.setup_micros.map_or("-".to_string(), |m| format!("{:.1}", m as f64 / 1000.0)),
        shielded.prove_micros as f64 / 1000.0,
        shielded.verify_micros as f64 / 1000.0,
        shielded.proof_bytes,
    )
}
