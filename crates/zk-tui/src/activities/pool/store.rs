//! Where the Toy Shielded Pool lives on disk, which proof system it uses, and the pool a TUI
//! session keeps open between commands.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::SystemTime;

use zk_core::Control;
use zk_pool::{Pool, PoolError};

use super::PoolSystem;

/// The file, inside the program's `pool` directory, naming the proof system in use.
const SYSTEM_FILE: &str = "system";

/// The program's data directory: `--data-dir` when given; otherwise `$XDG_DATA_HOME/nmtzk` or
/// `$HOME/.local/share/nmtzk`, and `$HOME/Library/Application Support/nmtzk` on macOS. `None`
/// when no directory can be named.
pub fn data_root(explicit: Option<&Path>) -> Option<PathBuf> {
    resolve_root(explicit, cfg!(target_os = "macos"), |name| std::env::var_os(name))
}

/// [`data_root`] with the platform and the environment passed in, so both can be tested.
fn resolve_root(
    explicit: Option<&Path>,
    macos: bool,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    if let Some(dir) = explicit {
        return Some(dir.to_path_buf());
    }
    // The XDG specification says a relative value is invalid and must be ignored.
    let absolute = |name: &str| env(name).map(PathBuf::from).filter(|path| path.is_absolute());
    let home = absolute("HOME");
    let base = if macos {
        home?.join("Library").join("Application Support")
    } else {
        absolute("XDG_DATA_HOME").or_else(|| home.map(|home| home.join(".local").join("share")))?
    };
    Some(base.join("nmtzk"))
}

/// The directory holding every pool: one subdirectory per proof system, and the system file.
fn pools(root: &Path) -> PathBuf {
    root.join("pool")
}

/// Where the pool proved by `system` keeps its ledger and wallets.
pub fn pool_dir(root: &Path, system: PoolSystem) -> PathBuf {
    pools(root).join(system.key())
}

/// The proof system chosen with `use`; Groth16 until one is chosen.
///
/// Only a missing file means nothing was chosen. A file that cannot be read or names no system is
/// an error, because falling back to Groth16 would quietly show another pool's wallets.
pub fn chosen_system(root: &Path) -> Result<PoolSystem, PoolError> {
    let path = pools(root).join(SYSTEM_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => PoolSystem::from_key(text.trim()).ok_or_else(|| {
            PoolError::Corrupt(format!(
                "{} names no proof system: {:?}",
                path.display(),
                text.trim()
            ))
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(PoolSystem::default()),
        Err(error) => Err(PoolError::Storage(format!("reading {}: {error}", path.display()))),
    }
}

/// Remembers `system` for the next command, replacing the file in one rename so that a reader never
/// finds it half written.
pub fn choose_system(root: &Path, system: PoolSystem) -> Result<(), PoolError> {
    let dir = pools(root);
    let path = dir.join(SYSTEM_FILE);
    let temp = dir.join(format!("{SYSTEM_FILE}.{}.tmp", std::process::id()));
    let storage = |what: &str, at: &Path, error: std::io::Error| {
        PoolError::Storage(format!("{what} {}: {error}", at.display()))
    };
    fs::create_dir_all(&dir).map_err(|e| storage("creating", &dir, e))?;
    fs::write(&temp, format!("{}\n", system.key())).map_err(|e| storage("writing", &temp, e))?;
    fs::rename(&temp, &path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        storage("renaming over", &path, e)
    })
}

/// A pool of either proof system the museum offers.
pub(crate) enum AnyPool {
    Groth16(Pool<sys_groth16::Field>),
    Halo2(Pool<sys_halo2::Field>),
}

impl AnyPool {
    fn open(system: PoolSystem, dir: &Path) -> Result<AnyPool, PoolError> {
        Ok(match system {
            PoolSystem::Groth16 => AnyPool::Groth16(Pool::groth16(dir)?),
            PoolSystem::Halo2 => AnyPool::Halo2(Pool::halo2(dir)?),
        })
    }
}

/// Sizes and modification times of the files a pool reads, so a pool kept open notices when
/// another process (the command line) changed them.
type Stamp = Vec<(PathBuf, u64, Option<SystemTime>)>;

fn stamp(dir: &Path) -> Stamp {
    let mut files = vec![dir.join("ledger.json")];
    if let Ok(entries) = fs::read_dir(dir.join("wallets")) {
        files.extend(entries.filter_map(Result::ok).map(|entry| entry.path()));
    }
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let meta = fs::metadata(&path).ok()?;
            Some((path, meta.len(), meta.modified().ok()))
        })
        .collect()
}

struct Kept {
    dir: PathBuf,
    system: PoolSystem,
    stamp: Stamp,
    pool: AnyPool,
}

/// The pool one TUI session keeps open, so its proof system's keys are built once rather than for
/// every command. It is opened again whenever its files changed on disk since it last wrote them.
#[derive(Clone, Default)]
pub struct PoolCache(Arc<Mutex<Option<Kept>>>);

impl core::fmt::Debug for PoolCache {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PoolCache").finish_non_exhaustive()
    }
}

impl PoolCache {
    fn lock(&self) -> MutexGuard<'_, Option<Kept>> {
        // A panic while the pool was held may have left it half changed: forget it and reopen.
        self.0.lock().unwrap_or_else(|poisoned| {
            let mut guard = poisoned.into_inner();
            *guard = None;
            guard
        })
    }

    /// Runs `act` on the pool for `system` in `dir`, opening it first unless it is already open
    /// and unchanged on disk, and keeps it open afterwards.
    ///
    /// The pool directory's lock is held from before the files are compared until after `act` has
    /// written them, so a command line and a screen using one pool take turns; `waiting` is called
    /// if the other one is in the middle of something.
    pub(crate) fn with<T>(
        &self,
        (system, dir): (PoolSystem, &Path),
        control: &Control,
        waiting: impl FnOnce(),
        act: impl FnOnce(&mut AnyPool) -> Result<T, PoolError>,
    ) -> Result<T, PoolError> {
        let _files = zk_pool::lock(dir, control, waiting)?;
        let mut guard = self.lock();
        let reusable = guard.as_ref().is_some_and(|kept| {
            kept.system == system && kept.dir == dir && kept.stamp == stamp(dir)
        });
        let mut pool = match guard.take() {
            Some(kept) if reusable => kept.pool,
            _ => AnyPool::open(system, dir)?,
        };
        match &mut pool {
            AnyPool::Groth16(pool) => pool.set_control(control.clone()),
            AnyPool::Halo2(pool) => pool.set_control(control.clone()),
        }
        let result = act(&mut pool);
        *guard = Some(Kept { dir: dir.to_path_buf(), system, stamp: stamp(dir), pool });
        result
    }

    /// Deletes the ledger and wallets of the pool for `system` in `dir` without reading them, so a
    /// pool whose files are damaged can be started over. A pool kept open for it forgets its world
    /// but keeps its keys.
    pub(crate) fn reset(
        &self,
        (system, dir): (PoolSystem, &Path),
        control: &Control,
        waiting: impl FnOnce(),
    ) -> Result<(), PoolError> {
        let _files = zk_pool::lock(dir, control, waiting)?;
        let mut guard = self.lock();
        match guard.as_mut() {
            Some(kept) if kept.system == system && kept.dir == dir => {
                match &mut kept.pool {
                    AnyPool::Groth16(pool) => pool.reset()?,
                    AnyPool::Halo2(pool) => pool.reset()?,
                }
                kept.stamp = stamp(dir);
            }
            _ => zk_pool::remove_pool(dir)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |name| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| value.into())
    }

    #[test]
    fn the_data_directory_follows_the_platform_and_the_flag() {
        let linux = env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "/data")]);
        assert_eq!(resolve_root(None, false, &linux), Some(PathBuf::from("/data/nmtzk")));
        let no_xdg = env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "relative")]);
        assert_eq!(
            resolve_root(None, false, &no_xdg),
            Some(PathBuf::from("/home/a/.local/share/nmtzk"))
        );
        assert_eq!(
            resolve_root(None, true, &linux),
            Some(PathBuf::from("/home/a/Library/Application Support/nmtzk"))
        );
        assert_eq!(resolve_root(None, false, env(&[])), None);
        let flag = Path::new("/tmp/x");
        assert_eq!(resolve_root(Some(flag), false, env(&[])), Some(PathBuf::from("/tmp/x")));
        assert_eq!(pool_dir(flag, PoolSystem::Halo2), PathBuf::from("/tmp/x/pool/halo2"));
    }

    #[test]
    fn only_a_missing_system_file_means_groth16() {
        let root = std::env::temp_dir().join(format!("nmtzk-system-file-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(chosen_system(&root), Ok(PoolSystem::Groth16));
        assert_eq!(choose_system(&root, PoolSystem::Halo2), Ok(()));
        assert_eq!(chosen_system(&root), Ok(PoolSystem::Halo2));
        for damaged in ["Halo2\n", "", "halo2 \u{ff}"] {
            fs::write(pools(&root).join(SYSTEM_FILE), damaged).ok();
            assert!(matches!(chosen_system(&root), Err(PoolError::Corrupt(_))), "{damaged:?}");
        }
        let entries = fs::read_dir(pools(&root)).map(|dir| dir.count()).unwrap_or_default();
        assert_eq!(entries, 1, "no temporary file is left beside the system file");
        let _ = fs::remove_dir_all(&root);
    }
}
