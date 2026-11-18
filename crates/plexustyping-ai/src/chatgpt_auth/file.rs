//! The private credential file: owner-only, replaced atomically.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::AuthError;

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);
/// One writer at a time: Windows refuses concurrent renames onto the same file.
static WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn storage(err: std::io::Error) -> AuthError {
    AuthError::Storage {
        message: err.to_string(),
    }
}

/// The file's text, `None` when it does not exist.
pub(super) fn load(path: &Path) -> Result<Option<String>, AuthError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(storage(err)),
    }
}

/// Temporary names tried before giving up when every one is already taken.
const TMP_ATTEMPTS: u32 = 16;

/// Writes `contents` to a private temporary file next to `path`, then renames it over `path`.
pub(super) fn save(path: &Path, contents: &str) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage)?;
    }
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut attempt = 0;
    let (tmp, file) = loop {
        let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
        let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        tmp_name.push(format!(".tmp-{}-{seq}", std::process::id()));
        let tmp = path.with_file_name(tmp_name);
        match create_private(&tmp) {
            Ok(file) => break (tmp, file),
            // Someone else's file: never written through, never removed.
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                attempt += 1;
                if attempt >= TMP_ATTEMPTS {
                    return Err(storage(err));
                }
            }
            Err(err) => return Err(storage(err)),
        }
    };
    let result = write_all(file, contents.as_bytes())
        .and_then(|()| replace_file(&tmp, path))
        .and_then(|()| sync_parent(path));
    if let Err(err) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(storage(err));
    }
    Ok(())
}

/// Creates a new file at `path` readable and writable by its owner only (0600
/// on Unix; on Windows the per-user app config folder's permissions apply). An
/// existing file or symlink there is refused, so its permissions never apply.
fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

fn write_all(mut file: std::fs::File, contents: &[u8]) -> std::io::Result<()> {
    file.write_all(contents)?;
    file.sync_all()
}

/// Makes the rename itself durable (Unix; Windows has no directory handles for this).
fn sync_parent(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Renames `from` over `to`, retrying briefly on Windows while another process holds `to`.
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            Err(err)
                if cfg!(windows)
                    && err.kind() == std::io::ErrorKind::PermissionDenied
                    && attempt < 10 =>
            {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(20 * attempt));
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "plexustyping-file-{name}-{}-{}",
            std::process::id(),
            TMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_existing_temporary_name_is_skipped_and_left_untouched() {
        let dir = scratch("taken");
        let path = dir.join("auth.json");
        // Every name the next saves could pick is taken by someone else's file.
        let next = TMP_SEQ.load(Ordering::Relaxed);
        let taken: Vec<_> = (next..next + 4)
            .map(|seq| dir.join(format!("auth.json.tmp-{}-{seq}", std::process::id())))
            .collect();
        for file in &taken {
            std::fs::write(file, "theirs").unwrap();
        }

        save(&path, "secret").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "secret");
        for file in &taken {
            assert_eq!(std::fs::read_to_string(file).unwrap(), "theirs");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
