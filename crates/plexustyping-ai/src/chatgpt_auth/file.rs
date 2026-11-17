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

/// Writes `contents` to a private temporary file next to `path`, then renames it over `path`.
pub(super) fn save(path: &Path, contents: &str) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage)?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    tmp_name.push(format!(".tmp-{}-{seq}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);

    let _guard = WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let result = write_private(&tmp, contents.as_bytes()).and_then(|()| replace_file(&tmp, path));
    if let Err(err) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(storage(err));
    }
    Ok(())
}

/// Creates `path` readable and writable by its owner only (0600 on Unix; on
/// Windows the per-user app config folder's permissions apply).
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
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
