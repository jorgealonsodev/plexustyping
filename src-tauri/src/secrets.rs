//! AI provider keys, kept only in the system keychain (Keychain, Credential
//! Manager or Secret Service), never in plain text (S2).
//!
//! Adapted from PlexusTerm. Keys never leave the backend: the frontend can only
//! save one, ask whether one exists, or delete it. Errors never carry secret bytes.

use std::collections::HashMap;
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;

/// Keychain service under which every PlexusTyping secret is stored.
pub const SERVICE: &str = "com.plexustyping.app";

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SecretError {
    #[error("the system keychain is not available")]
    Unavailable,
    #[error("system keychain error: {message}")]
    Keyring { message: String },
}

/// Where keys live. Implementations may block (D-Bus round trips, unlock
/// prompts), so async code goes through [`run`].
pub trait SecretStore: Send + Sync {
    /// Whether the store can be used at all (e.g. a Secret Service is running).
    fn available(&self) -> bool;
    fn get(&self, account: &str) -> Result<Option<String>, SecretError>;
    /// Creates or replaces the secret of `account`; `label` is what keychain UIs show.
    fn set(&self, account: &str, label: &str, secret: &str) -> Result<(), SecretError>;
    /// Deletes the secret of `account`; deleting a missing secret succeeds.
    fn delete(&self, account: &str) -> Result<(), SecretError>;
}

pub type SharedSecrets = Arc<dyn SecretStore>;

/// Runs a store operation on the blocking pool so the async runtime never waits on the keychain.
pub async fn run<T: Send + 'static>(
    store: &SharedSecrets,
    op: impl FnOnce(&dyn SecretStore) -> Result<T, SecretError> + Send + 'static,
) -> Result<T, SecretError> {
    let store = store.clone();
    tokio::task::spawn_blocking(move || op(&*store))
        .await
        .map_err(|err| SecretError::Keyring {
            message: err.to_string(),
        })?
}

/// A store that is never available: nothing is ever written anywhere.
pub struct NoStore;

impl SecretStore for NoStore {
    fn available(&self) -> bool {
        false
    }
    fn get(&self, _account: &str) -> Result<Option<String>, SecretError> {
        Err(SecretError::Unavailable)
    }
    fn set(&self, _account: &str, _label: &str, _secret: &str) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
    fn delete(&self, _account: &str) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
}

/// The user's system keychain through `keyring`.
pub struct KeyringStore {
    service: String,
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::with_service(SERVICE)
    }
}

/// Maps keychain errors to messages that never carry secret bytes.
fn keyring_error(err: keyring_core::Error) -> SecretError {
    let message = match err {
        keyring_core::Error::BadEncoding(_) | keyring_core::Error::BadDataFormat(..) => {
            "the saved secret is malformed".to_string()
        }
        keyring_core::Error::Ambiguous(items) => {
            format!("{} keychain items match this key", items.len())
        }
        keyring_core::Error::NoDefaultStore => return SecretError::Unavailable,
        other => other.to_string(),
    };
    SecretError::Keyring { message }
}

impl KeyringStore {
    pub fn with_service(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    fn entry(
        &self,
        account: &str,
        label: Option<&str>,
    ) -> Result<keyring_core::Entry, SecretError> {
        // Installs the platform's store as keyring-core's default store, once.
        if keyring::Entry::store_status().is_err() {
            return Err(SecretError::Unavailable);
        }
        let modifiers = label.and_then(|label| entry_modifiers(std::env::consts::OS, label));
        let entry = match modifiers {
            Some(modifiers) => {
                keyring_core::Entry::new_with_modifiers(&self.service, account, &modifiers)
            }
            None => keyring_core::Entry::new(&self.service, account),
        };
        entry.map_err(keyring_error)
    }
}

/// The modifiers that give a new keychain item `label` on platform `os`. Only
/// the Secret Service store (Unix except Apple and Android) knows `label`; the
/// Windows and Apple stores reject unknown modifiers.
fn entry_modifiers<'a>(os: &str, label: &'a str) -> Option<HashMap<&'static str, &'a str>> {
    match os {
        "windows" | "macos" | "ios" | "android" => None,
        _ => Some(HashMap::from([("label", label)])),
    }
}

impl SecretStore for KeyringStore {
    fn available(&self) -> bool {
        keyring::Entry::store_status().is_ok()
    }

    fn get(&self, account: &str) -> Result<Option<String>, SecretError> {
        match self.entry(account, None)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(err) => Err(keyring_error(err)),
        }
    }

    fn set(&self, account: &str, label: &str, secret: &str) -> Result<(), SecretError> {
        // Updated in place: deleting first could lose the old key if the write failed.
        self.entry(account, Some(label))?
            .set_password(secret)
            .map_err(keyring_error)
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        match self.entry(account, None)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(err) => Err(keyring_error(err)),
        }
    }
}

/// In-memory store for tests; it also records labels.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<HashMap<String, (String, String)>>,
}

#[cfg(test)]
impl MemoryStore {
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn label(&self, account: &str) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        entries.get(account).map(|(label, _)| label.clone())
    }
}

#[cfg(test)]
impl SecretStore for MemoryStore {
    fn available(&self) -> bool {
        true
    }
    fn get(&self, account: &str) -> Result<Option<String>, SecretError> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.get(account).map(|(_, secret)| secret.clone()))
    }
    fn set(&self, account: &str, label: &str, secret: &str) -> Result<(), SecretError> {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(account.into(), (label.into(), secret.into()));
        Ok(())
    }
    fn delete(&self, account: &str) -> Result<(), SecretError> {
        self.entries.lock().unwrap().remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_uses_the_blocking_pool_on_a_current_thread_runtime() {
        let store: SharedSecrets = MemoryStore::shared();
        run(&store, |s| s.set("ai:claude", "label", "k"))
            .await
            .unwrap();
        let got = run(&store, |s| s.get("ai:claude")).await.unwrap();
        assert_eq!(got.as_deref(), Some("k"));
        run(&store, |s| s.delete("ai:claude")).await.unwrap();
        run(&store, |s| s.delete("ai:claude")).await.unwrap();
        assert_eq!(run(&store, |s| s.get("ai:claude")).await.unwrap(), None);
    }

    #[test]
    fn only_secret_service_platforms_get_a_label_modifier() {
        let label = |os| entry_modifiers(os, "PlexusTyping AI key (OpenCode Go)");
        for os in ["linux", "freebsd", "openbsd"] {
            let expected = HashMap::from([("label", "PlexusTyping AI key (OpenCode Go)")]);
            assert_eq!(label(os), Some(expected), "{os}");
        }
        for os in ["windows", "macos", "ios", "android"] {
            assert_eq!(label(os), None, "{os}");
        }
    }

    #[test]
    fn no_store_is_unavailable_and_never_stores() {
        assert!(!NoStore.available());
        assert_eq!(NoStore.set("a", "l", "k"), Err(SecretError::Unavailable));
        assert_eq!(NoStore.get("a"), Err(SecretError::Unavailable));
    }

    #[test]
    fn keyring_errors_never_carry_secret_bytes() {
        let err = keyring_error(keyring_core::Error::BadEncoding(b"hunter2".to_vec()));
        let shown = format!("{err} {err:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
        assert_eq!(
            keyring_error(keyring_core::Error::NoDefaultStore),
            SecretError::Unavailable
        );
    }

    /// Writes to the real keychain under a throwaway service and removes what it created.
    /// Run with `cargo test -p plexustyping-app -- --ignored real_keychain`.
    #[test]
    #[ignore = "touches the user's real keychain"]
    fn real_keychain_round_trip() {
        let nonce = std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos();
        let store = KeyringStore::with_service(&format!("com.plexustyping.test.{nonce}"));
        assert!(store.available(), "no system keychain on this machine");
        store.set("ai:claude", "PlexusTyping test", "k").unwrap();
        let got = store.get("ai:claude");
        store.delete("ai:claude").unwrap();
        assert_eq!(got.unwrap().as_deref(), Some("k"));
        assert_eq!(store.get("ai:claude").unwrap(), None);
    }
}
