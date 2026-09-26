//! Files: `ledger.json` and `wallets/<name>.json` in the caller's directory, each replaced atomically,
//! and the `lock` file that lets one process at a time read and change them.

use std::fs::{self, File, TryLockError};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use zk_core::{Control, SystemError};

use crate::error::PoolError;

/// The ledger's file name inside the pool directory.
pub(crate) const LEDGER_FILE: &str = "ledger.json";

/// The directory, inside the pool directory, holding one file per wallet.
pub(crate) const WALLETS_DIR: &str = "wallets";

/// The file, inside the pool directory, that a process holds while it reads and changes the pool.
pub(crate) const LOCK_FILE: &str = "lock";

const EXTENSION: &str = "json";
const MAX_NAME_BYTES: usize = 32;

/// How often a process waiting for the lock looks again, and whether it was asked to stop.
const LOCK_RETRY: Duration = Duration::from_millis(20);

/// Refuses names that could escape the wallets directory or collide with its temporary files.
pub(crate) fn check_name(name: &str) -> Result<(), PoolError> {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if name.is_empty() || name.len() > MAX_NAME_BYTES || !name.chars().all(allowed) {
        return Err(PoolError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Words the commands use in a wallet's place, which a new wallet therefore cannot be called.
const RESERVED_NAMES: [&str; 1] = ["new"];

/// [`check_name`], and refuses the words the commands reserve. Only new wallets are held to this,
/// so a wallet made before a word was reserved still opens.
pub(crate) fn check_new_name(name: &str) -> Result<(), PoolError> {
    check_name(name)?;
    if RESERVED_NAMES.contains(&name) {
        return Err(PoolError::ReservedName(name.to_string()));
    }
    Ok(())
}

/// Where a wallet's file lives; `name` must already have passed [`check_name`].
pub(crate) fn wallet_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(WALLETS_DIR).join(format!("{name}.{EXTENSION}"))
}

/// A temporary file beside `path` that no other write shares, `<name>.json.<process>-<n>.tmp`: two
/// processes writing the same file never write into one temporary file, and no `.tmp` name passes
/// [`check_name`], so a leftover is never mistaken for a wallet.
fn temp_path(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("{EXTENSION}.{}-{n}.tmp", std::process::id()))
}

/// Writes `value` as JSON to a temporary file beside `path`, flushes it to disk, then renames it
/// over `path`. A rename within one directory is atomic, so a reader, or a crash, sees either the
/// old file or the new one, never half of either; on any failure the temporary file is removed.
pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), PoolError> {
    let temp = temp_path(path);
    let written = write_temp(&temp, value).and_then(|()| {
        fs::rename(&temp, path).map_err(|e| format!("renaming over {}: {e}", path.display()))
    });
    if written.is_err() {
        // The temporary file may not exist; either way nothing else refers to it.
        let _ = fs::remove_file(&temp);
    } else if let Some(parent) = path.parent().and_then(|parent| File::open(parent).ok()) {
        // The rename itself reaches the disk with the directory. Best effort: the new file is
        // already complete, and a directory that cannot be synced loses at most this write.
        let _ = parent.sync_all();
    }
    written.map_err(PoolError::Storage)
}

fn write_temp<T: Serialize>(temp: &Path, value: &T) -> Result<(), String> {
    let context = |e: &dyn std::fmt::Display| format!("writing {}: {e}", temp.display());
    let file = File::options().write(true).create_new(true).open(temp).map_err(|e| context(&e))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|e| context(&e))?;
    writer.write_all(b"\n").map_err(|e| context(&e))?;
    let file = writer.into_inner().map_err(|e| context(&e))?;
    file.sync_all().map_err(|e| context(&e))
}

/// Reads a JSON file, or `None` when it does not exist.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, PoolError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(PoolError::Storage(format!("reading {}: {e}", path.display()))),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| PoolError::Corrupt(format!("{}: {e}", path.display())))
}

/// Names of the wallet files present, sorted; temporary and foreign files are ignored.
pub(crate) fn wallet_names(dir: &Path) -> Result<Vec<String>, PoolError> {
    let wallets = dir.join(WALLETS_DIR);
    let entries = match fs::read_dir(&wallets) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(PoolError::Storage(format!("listing {}: {e}", wallets.display()))),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| PoolError::Storage(format!("listing wallets: {e}")))?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str().and_then(|n| n.strip_suffix(".json")) else {
            continue;
        };
        if check_name(name).is_ok() {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Removes the ledger, every wallet file and any temporary file a stopped write left behind, and
/// nothing else in `dir`. No file is read, so a pool whose files are damaged can still be emptied.
pub(crate) fn remove_pool_files(dir: &Path) -> Result<(), PoolError> {
    let mut files = vec![dir.join(LEDGER_FILE)];
    files.extend(wallet_names(dir)?.iter().map(|name| wallet_path(dir, name)));
    files.extend(temp_files(dir));
    files.extend(temp_files(&dir.join(WALLETS_DIR)));
    for file in files {
        match fs::remove_file(&file) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => {
                return Err(PoolError::Storage(format!("removing {}: {e}", file.display())));
            }
        }
    }
    Ok(())
}

/// Temporary files [`write_json`] left in `dir` when a process stopped between writing and renaming.
fn temp_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            name.ends_with(".tmp") && name.contains(&format!(".{EXTENSION}."))
        })
        .collect()
}

/// The pool directory's lock, held until dropped.
///
/// Every command reads the pool, changes it in memory and writes it back, so two processes working
/// on one directory at once would each write back what they read and one of them would lose its
/// transaction. Holding this from the first read to the last write makes them take turns.
#[derive(Debug)]
pub struct PoolLock {
    /// `None` when the directory is read-only: nothing can be written there, so nothing is lost.
    _file: Option<File>,
}

/// Waits for, then takes, the lock on the pool kept in `dir`, creating the directory first.
///
/// `waiting` is called once if another process holds the lock, so a screen can say why nothing
/// happens yet; a stop asked for through `control` ends the wait with a cancellation. In a
/// read-only directory the pool can still be read, and the lock is skipped.
pub fn lock(dir: &Path, control: &Control, waiting: impl FnOnce()) -> Result<PoolLock, PoolError> {
    create_dirs(dir)?;
    let path = dir.join(LOCK_FILE);
    let opened = File::options().create(true).truncate(false).write(true).open(&path);
    let file = match opened {
        Ok(file) => file,
        Err(e)
            if matches!(e.kind(), ErrorKind::ReadOnlyFilesystem | ErrorKind::PermissionDenied) =>
        {
            match File::open(&path) {
                Ok(file) => file,
                Err(_) => return Ok(PoolLock { _file: None }),
            }
        }
        Err(e) => return Err(PoolError::Storage(format!("opening {}: {e}", path.display()))),
    };
    let mut waiting = Some(waiting);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(PoolLock { _file: Some(file) }),
            Err(TryLockError::WouldBlock) => {
                if let Some(waiting) = waiting.take() {
                    waiting();
                }
                if control.is_cancelled() {
                    return Err(PoolError::System(SystemError::Cancelled));
                }
                thread::sleep(LOCK_RETRY);
            }
            Err(TryLockError::Error(e)) => {
                return Err(PoolError::Storage(format!("locking {}: {e}", path.display())));
            }
        }
    }
}

/// Deletes the ledger and every wallet file of the pool kept in `dir`, without reading any of them,
/// so that a pool whose files are damaged can still be started over. Take [`lock`] first.
pub fn remove_pool(dir: &Path) -> Result<(), PoolError> {
    remove_pool_files(dir)
}

/// Creates the pool directory and its wallets directory.
pub(crate) fn create_dirs(dir: &Path) -> Result<(), PoolError> {
    let wallets = dir.join(WALLETS_DIR);
    fs::create_dir_all(&wallets)
        .map_err(|e| PoolError::Storage(format!("creating {}: {e}", wallets.display())))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use serde::ser::Error as _;

    use super::*;
    use crate::Pool;

    /// Serializes its first field, then fails, so the temporary file is left half written.
    #[derive(Serialize)]
    struct FailsHalfway {
        pool_balance: u64,
        broken: Unserializable,
    }

    struct Unserializable;

    impl Serialize for Unserializable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(S::Error::custom("refused on purpose"))
        }
    }

    #[test]
    fn a_failed_serialization_leaves_the_old_ledger_intact() {
        let dir = std::env::temp_dir().join(format!("zk-pool-atomic-{}", std::process::id()));
        let mut pool = Pool::groth16(&dir).unwrap();
        pool.new_wallet("alice").unwrap();
        pool.faucet("alice", 40).unwrap();
        let path = dir.join(LEDGER_FILE);
        let before = fs::read(&path).unwrap();
        let half = FailsHalfway { pool_balance: 1, broken: Unserializable };
        assert!(matches!(write_json(&path, &half), Err(PoolError::Storage(_))));
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(temp_files(&dir).is_empty(), "the half-written temporary file was removed");
        let reopened = Pool::groth16(&dir).unwrap();
        assert_eq!(reopened.ledger().transparent.get("alice"), Some(&40));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn every_write_has_a_temporary_file_of_its_own() {
        let path = Path::new("/pool/ledger.json");
        let (first, second) = (temp_path(path), temp_path(path));
        assert_ne!(first, second);
        let name = first.file_name().and_then(|name| name.to_str()).unwrap();
        assert!(name.starts_with("ledger.json.") && name.ends_with(".tmp"), "{name}");
        assert!(check_name(name.trim_end_matches(".json")).is_err());
    }

    #[test]
    fn a_held_lock_makes_the_next_process_wait_and_a_stop_ends_the_wait() {
        let dir = std::env::temp_dir().join(format!("zk-pool-lock-{}", std::process::id()));
        let held = lock(&dir, &Control::new(), || {}).unwrap();
        let control = Control::new();
        control.cancel();
        let mut waited = false;
        let second = lock(&dir, &control, || waited = true);
        assert_eq!(second.unwrap_err(), PoolError::System(SystemError::Cancelled));
        assert!(waited, "the second caller was told it had to wait");
        drop(held);
        let mut waited = false;
        assert!(lock(&dir, &control, || waited = true).is_ok());
        assert!(!waited, "a free lock is taken at once");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_damaged_pool_is_removed_without_being_read() {
        let dir = std::env::temp_dir().join(format!("zk-pool-remove-{}", std::process::id()));
        create_dirs(&dir).unwrap();
        fs::write(dir.join(LEDGER_FILE), "{ not json").unwrap();
        fs::write(wallet_path(&dir, "alice"), "").unwrap();
        fs::write(dir.join("ledger.json.1-0.tmp"), "half").unwrap();
        fs::write(dir.join("notes.txt"), "kept").unwrap();
        remove_pool(&dir).unwrap();
        assert!(!dir.join(LEDGER_FILE).exists() && !wallet_path(&dir, "alice").exists());
        assert!(!dir.join("ledger.json.1-0.tmp").exists());
        assert!(dir.join("notes.txt").exists(), "files the pool did not write are left alone");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn names_that_could_leave_the_wallets_directory_are_refused() {
        for bad in ["", "../alice", "Alice", "a/b", "a.json", &"x".repeat(33)] {
            assert_eq!(check_name(bad), Err(PoolError::InvalidName(bad.to_string())));
        }
        assert_eq!(check_name("carol-2_x"), Ok(()));
        assert_eq!(check_new_name("new"), Err(PoolError::ReservedName("new".to_string())));
        assert_eq!(check_new_name("newt"), Ok(()));
    }
}
