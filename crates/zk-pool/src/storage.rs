//! Files: `ledger.json` and `wallets/<name>.json` in the caller's directory, each replaced atomically.

use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::PoolError;

/// The ledger's file name inside the pool directory.
pub(crate) const LEDGER_FILE: &str = "ledger.json";

/// The directory, inside the pool directory, holding one file per wallet.
pub(crate) const WALLETS_DIR: &str = "wallets";

const EXTENSION: &str = "json";
const MAX_NAME_BYTES: usize = 32;

/// Refuses names that could escape the wallets directory or collide with its temporary files.
pub(crate) fn check_name(name: &str) -> Result<(), PoolError> {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if name.is_empty() || name.len() > MAX_NAME_BYTES || !name.chars().all(allowed) {
        return Err(PoolError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Where a wallet's file lives; `name` must already have passed [`check_name`].
pub(crate) fn wallet_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(WALLETS_DIR).join(format!("{name}.{EXTENSION}"))
}

/// The temporary file a write goes to before it replaces `path`.
fn temp_path(path: &Path) -> PathBuf {
    path.with_extension(format!("{EXTENSION}.tmp"))
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
    }
    written.map_err(PoolError::Storage)
}

fn write_temp<T: Serialize>(temp: &Path, value: &T) -> Result<(), String> {
    let context = |e: &dyn std::fmt::Display| format!("writing {}: {e}", temp.display());
    let file = File::create(temp).map_err(|e| context(&e))?;
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

/// Removes the ledger and every wallet file this pool wrote, and nothing else in `dir`.
pub(crate) fn remove_pool_files(dir: &Path) -> Result<(), PoolError> {
    let mut files = vec![dir.join(LEDGER_FILE)];
    files.extend(wallet_names(dir)?.iter().map(|name| wallet_path(dir, name)));
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
        assert!(!temp_path(&path).exists());
        let reopened = Pool::groth16(&dir).unwrap();
        assert_eq!(reopened.ledger().transparent.get("alice"), Some(&40));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn names_that_could_leave_the_wallets_directory_are_refused() {
        for bad in ["", "../alice", "Alice", "a/b", "a.json", &"x".repeat(33)] {
            assert_eq!(check_name(bad), Err(PoolError::InvalidName(bad.to_string())));
        }
        assert_eq!(check_name("carol-2_x"), Ok(()));
    }
}
