//! API keys for Labs › Clean up on your own server, one per provider, kept
//! out of settings.json so `hex settings` and backups never show them.
//!
//! macOS keeps them in the login keychain and Windows in Credential Manager
//! (`keyring`). Linux, and either of them when its store refuses, uses an
//! owner-only `ai-cleanup-keys.json` in Hex's data folder, as OpenCode keeps
//! its own `auth.json`. A key read once is held in memory until it changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use super::Provider;

const FILE: &str = "ai-cleanup-keys.json";

/// Keys read or written this run; `None` remembers that there is none.
static CACHE: Mutex<Option<HashMap<&'static str, Option<String>>>> = Mutex::new(None);

/// The saved key for `provider`, if any.
pub fn get(provider: Provider) -> Option<String> {
    let name = provider.key_name();
    if let Ok(mut cache) = CACHE.lock()
        && let Some(known) = cache.get_or_insert_with(HashMap::new).get(name)
    {
        return known.clone();
    }
    let key = store::get(name).or_else(|| file::get(name));
    if let Ok(mut cache) = CACHE.lock() {
        cache
            .get_or_insert_with(HashMap::new)
            .insert(name, key.clone());
    }
    key
}

/// Whether `provider` has a key saved.
pub fn has(provider: Provider) -> bool {
    get(provider).is_some()
}

/// Saves `key` for `provider`; an empty key removes it.
pub fn set(provider: Provider, key: &str) -> Result<(), String> {
    let name = provider.key_name();
    let key = key.trim();
    let result = if key.is_empty() {
        let stored = store::remove(name);
        let filed = file::set(name, None);
        stored.and(filed)
    } else {
        match store::set(name, key) {
            // Saved in the OS store: drop any older copy from the file.
            Ok(()) => file::set(name, None),
            Err(error) => {
                tracing::warn!(%error, "the credential store refused the AI cleanup key; using the owner-only file");
                file::set(name, Some(key))
            }
        }
    };
    if let Ok(mut cache) = CACHE.lock() {
        cache
            .get_or_insert_with(HashMap::new)
            .insert(name, (!key.is_empty()).then(|| key.to_owned()));
    }
    result
}

fn file_path() -> Option<PathBuf> {
    crate::app_paths::support_dir()
        .ok()
        .map(|folder| folder.join(FILE))
}

/// The OS credential store: Keychain on macOS, Credential Manager on
/// Windows. Tests never touch it.
#[cfg(all(any(target_os = "macos", target_os = "windows"), not(test)))]
mod store {
    fn service() -> &'static str {
        if crate::app_paths::is_macos_test_build() {
            "Hex Test AI cleanup"
        } else {
            "Hex AI cleanup"
        }
    }

    fn entry(name: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(service(), name).map_err(|error| error.to_string())
    }

    pub fn get(name: &str) -> Option<String> {
        match entry(name).ok()?.get_password() {
            Ok(key) if !key.trim().is_empty() => Some(key),
            Ok(_) | Err(keyring::Error::NoEntry) => None,
            Err(error) => {
                tracing::warn!(%error, "couldn't read the AI cleanup key from the credential store");
                None
            }
        }
    }

    pub fn set(name: &str, key: &str) -> Result<(), String> {
        entry(name)?
            .set_password(key)
            .map_err(|error| error.to_string())
    }

    pub fn remove(name: &str) -> Result<(), String> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// No OS store (Linux, and tests): every key goes to the owner-only file.
#[cfg(any(not(any(target_os = "macos", target_os = "windows")), test))]
mod store {
    pub fn get(_name: &str) -> Option<String> {
        None
    }

    pub fn set(_name: &str, _key: &str) -> Result<(), String> {
        Err("no credential store on this system".into())
    }

    pub fn remove(_name: &str) -> Result<(), String> {
        Ok(())
    }
}

/// `ai-cleanup-keys.json`: `{"openrouter": "sk-…"}`, readable by its owner only.
mod file {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    pub fn get(name: &str) -> Option<String> {
        get_at(&super::file_path()?, name)
    }

    pub fn set(name: &str, key: Option<&str>) -> Result<(), String> {
        let path = super::file_path().ok_or("the VibeHex data folder is unavailable")?;
        set_at(&path, name, key)
    }

    pub(super) fn get_at(path: &Path, name: &str) -> Option<String> {
        read(path).remove(name).filter(|key| !key.trim().is_empty())
    }

    fn read(path: &Path) -> BTreeMap<String, String> {
        fs::read(path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    pub(super) fn set_at(path: &Path, name: &str, key: Option<&str>) -> Result<(), String> {
        let mut keys = read(path);
        let changed = match key {
            Some(key) => keys.insert(name.into(), key.into()).as_deref() != Some(key),
            None => keys.remove(name).is_some(),
        };
        if !changed {
            return Ok(());
        }
        if keys.is_empty() {
            return match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            };
        }
        write(path, &keys).map_err(|error| format!("couldn't save the key: {error}"))
    }

    fn write(path: &Path, keys: &BTreeMap<String, String>) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
        let _ = fs::remove_file(&temporary);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        // On Windows the file inherits the per-user data folder's ACL.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&temporary)?;
        let mut data = serde_json::to_vec_pretty(keys)?;
        data.push(b'\n');
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::file::{get_at, set_at};

    #[test]
    fn the_key_file_is_owner_only_and_forgets_removed_keys() {
        let folder =
            std::env::temp_dir().join(format!("hex-ai-cleanup-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let path = folder.join(super::FILE);
        assert_eq!(get_at(&path, "openrouter"), None);
        set_at(&path, "openrouter", Some("sk-or-1")).unwrap();
        set_at(&path, "custom", Some("lm-studio")).unwrap();
        assert_eq!(get_at(&path, "openrouter").as_deref(), Some("sk-or-1"));
        assert_eq!(get_at(&path, "custom").as_deref(), Some("lm-studio"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        set_at(&path, "openrouter", None).unwrap();
        assert_eq!(get_at(&path, "openrouter"), None);
        set_at(&path, "custom", None).unwrap();
        assert!(!path.exists(), "an empty key file is removed");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
