//! Secret storage backed by the OS Keychain (macOS/Windows/Linux Secret Service).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
// The v1 `keyring` crate re-exports `Entry`/`Error` and lazily selects the
// platform-native store on the first `Entry::new` (macOS Keychain / Windows
// Credential Manager / Linux Secret Service) — replacing the store-init
// helper that was removed in keyring 4.1.
use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "dev.translator.desktop";

static FALLBACK_SECRETS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn fallback_secrets_lock() -> &'static Mutex<()> {
    FALLBACK_SECRETS_LOCK.get_or_init(|| Mutex::new(()))
}

fn keyring_entry(service_id: &str) -> std::result::Result<Entry, String> {
    Entry::new(SERVICE_NAME, &format!("api_key:{service_id}")).map_err(|error| error.to_string())
}

/// Store an API key for a given service.
pub fn set_api_key(service_id: &str, api_key: &str) -> Result<()> {
    match keyring_entry(service_id) {
        Ok(entry) => match entry.set_password(api_key) {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::warn!(%service_id, %error, "keyring store failed; using local fallback");
                set_fallback_api_key(service_id, api_key)
            }
        },
        Err(error) => {
            tracing::warn!(%service_id, %error, "keyring entry failed; using local fallback");
            set_fallback_api_key(service_id, api_key)
        }
    }
}

/// Load an API key for a given service. Returns `Ok(None)` if not set.
pub fn get_api_key(service_id: &str) -> Result<Option<String>> {
    match keyring_entry(service_id) {
        Ok(entry) => match entry.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(KeyringError::NoEntry) => get_fallback_api_key(service_id),
            Err(e) => {
                tracing::warn!(%service_id, error = %e, "keyring read failed; trying local fallback");
                get_fallback_api_key(service_id)
            }
        },
        Err(error) => {
            tracing::warn!(%service_id, %error, "keyring entry failed; trying local fallback");
            get_fallback_api_key(service_id)
        }
    }
}

/// Cheap boolean probe used at dispatch time to decide whether a service has a
/// usable credential (BH-4.3). On macOS this uses a non-interactive attributes
/// query so settings status checks never trigger Keychain password prompts.
pub fn has_api_key(service_id: &str) -> Result<bool> {
    has_api_key_impl(service_id)
}

#[cfg(not(target_os = "macos"))]
fn has_api_key_impl(service_id: &str) -> Result<bool> {
    match keyring_entry(service_id) {
        Ok(entry) => match entry.get_password() {
            Ok(_) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(get_fallback_api_key(service_id)?.is_some()),
            Err(e) => {
                tracing::warn!(%service_id, error = %e, "keyring probe failed; trying local fallback");
                Ok(get_fallback_api_key(service_id)?.is_some())
            }
        },
        Err(error) => {
            tracing::warn!(%service_id, %error, "keyring entry failed; trying local fallback");
            Ok(get_fallback_api_key(service_id)?.is_some())
        }
    }
}

#[cfg(target_os = "macos")]
fn has_api_key_impl(service_id: &str) -> Result<bool> {
    match macos_keychain_entry_exists_without_prompt(service_id) {
        Ok(true) => Ok(true),
        Ok(false) => Ok(get_fallback_api_key(service_id)?.is_some()),
        Err(error) => {
            tracing::warn!(
                %service_id,
                %error,
                "non-interactive keyring probe failed; trying local fallback"
            );
            if get_fallback_api_key(service_id)?.is_some() {
                return Ok(true);
            }
            Err(anyhow::anyhow!(error))
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_keychain_entry_exists_without_prompt(
    service_id: &str,
) -> std::result::Result<bool, String> {
    use security_framework::item::{ItemClass, ItemSearchOptions};

    const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
    const ERR_SEC_INTERACTION_NOT_ALLOWED: i32 = -25308;

    let mut search = ItemSearchOptions::new();
    search
        .class(ItemClass::generic_password())
        .service(SERVICE_NAME)
        .account(&format!("api_key:{service_id}"))
        .load_attributes(true)
        .skip_authenticated_items(true)
        .limit(1);

    match search.search() {
        Ok(items) => Ok(!items.is_empty()),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(false),
        Err(error) if error.code() == ERR_SEC_INTERACTION_NOT_ALLOWED => {
            Err("keychain item exists but requires user interaction".to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Remove an API key for a given service.
pub fn delete_api_key(service_id: &str) -> Result<()> {
    match keyring_entry(service_id) {
        Ok(entry) => match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => delete_fallback_api_key(service_id),
            Err(e) => {
                tracing::warn!(%service_id, error = %e, "keyring delete failed; deleting local fallback");
                delete_fallback_api_key(service_id)
            }
        },
        Err(error) => {
            tracing::warn!(%service_id, %error, "keyring entry failed; deleting local fallback");
            delete_fallback_api_key(service_id)
        }
    }
}

fn fallback_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir on this platform")?;
    Ok(base.join("translator").join("secrets.json"))
}

fn read_fallback_secrets() -> Result<BTreeMap<String, String>> {
    let path = fallback_path()?;
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read fallback secrets: {}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn write_fallback_secrets(secrets: &BTreeMap<String, String>) -> Result<()> {
    let path = fallback_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create fallback secrets dir: {}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(secrets).context("serialize fallback secrets")?;
    std::fs::write(&path, text)
        .with_context(|| format!("write fallback secrets: {}", path.display()))?;
    Ok(())
}

fn set_fallback_api_key(service_id: &str, api_key: &str) -> Result<()> {
    let _guard = fallback_secrets_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("fallback secrets lock poisoned"))?;
    let mut secrets = read_fallback_secrets()?;
    secrets.insert(service_id.to_string(), api_key.to_string());
    write_fallback_secrets(&secrets)
}

fn get_fallback_api_key(service_id: &str) -> Result<Option<String>> {
    let _guard = fallback_secrets_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("fallback secrets lock poisoned"))?;
    Ok(read_fallback_secrets()?.get(service_id).cloned())
}

fn delete_fallback_api_key(service_id: &str) -> Result<()> {
    let _guard = fallback_secrets_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("fallback secrets lock poisoned"))?;
    let mut secrets = read_fallback_secrets()?;
    secrets.remove(service_id);
    write_fallback_secrets(&secrets)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    // Use a unique service id so this test does not collide with any real key
    // the user has stored. The test sets, probes, and cleans up its own entry.
    const MISSING_TEST_SVC: &str = "__translator_test_missing_bh43__";
    const ROUND_TRIP_TEST_SVC: &str = "__translator_test_roundtrip_bh43__";

    #[test]
    fn has_api_key_returns_false_for_missing_entry() {
        delete_api_key(MISSING_TEST_SVC).ok();
        match has_api_key(MISSING_TEST_SVC) {
            Ok(false) => {}
            Ok(true) => panic!("expected no key for unused service id"),
            Err(e) => panic!("keyring error (acceptable in sandboxed CI but not locally): {e}"),
        }
    }

    #[test]
    fn has_api_key_round_trips_with_set_and_delete() {
        // Acceptable to skip on CI runners without a working keyring backend.
        if has_api_key(ROUND_TRIP_TEST_SVC).is_err() {
            eprintln!("skipping: keyring backend unavailable in this environment");
            return;
        }
        delete_api_key(ROUND_TRIP_TEST_SVC).ok();
        assert!(
            !has_api_key(ROUND_TRIP_TEST_SVC).unwrap(),
            "should start unset"
        );

        set_api_key(ROUND_TRIP_TEST_SVC, "secret").expect("set");
        assert!(has_api_key(ROUND_TRIP_TEST_SVC).unwrap(), "should be set");
        assert_eq!(
            get_api_key(ROUND_TRIP_TEST_SVC).unwrap().as_deref(),
            Some("secret"),
            "get should round-trip the value"
        );

        delete_api_key(ROUND_TRIP_TEST_SVC).expect("delete");
        assert!(
            !has_api_key(ROUND_TRIP_TEST_SVC).unwrap(),
            "should be unset after delete"
        );
    }

    #[test]
    fn macos_keychain_probe_is_non_interactive() {
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/secrets.rs");
        let source = fs::read_to_string(source_path).expect("secrets.rs should be readable");
        let probe = source
            .split("fn macos_keychain_entry_exists_without_prompt")
            .nth(1)
            .expect("macOS keychain probe should exist");

        assert!(
            probe.contains(".load_attributes(true)")
                && probe.contains(".skip_authenticated_items(true)"),
            "macOS keychain status probe should load attributes only and suppress authentication UI",
        );
        assert!(
            !probe.contains("get_password()") && !probe.contains(".load_data(true)"),
            "macOS keychain status probe must not read secret data",
        );
    }

    #[test]
    fn secrets_do_not_use_removed_keyring_use_native_store() {
        // keyring 4.1 removed the `keyring::use_native_store` helper. The v1
        // feature (enabled by default) lazily selects the platform-native
        // store on the first `Entry::new` (macOS Keychain / Windows Credential
        // Manager / Linux Secret Service), so the manual store-init dance is
        // gone. This guards against reintroducing the removed API call, which
        // fails to compile against keyring 4.1 (E0425).
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/secrets.rs");
        let source = fs::read_to_string(source_path).expect("secrets.rs should be readable");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("secrets.rs should contain production source");

        assert!(
            !production_source.contains("use_native_store"),
            "secrets must not call keyring::use_native_store (removed in keyring 4.1); the v1 Entry initializes the store lazily",
        );
        assert!(
            production_source.contains("use keyring::"),
            "secrets should use the v1 `keyring` Entry/Error types so the platform-native store is initialized lazily",
        );
    }
}
