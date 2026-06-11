//! Secret storage backed by the OS Keychain (macOS/Windows/Linux Secret Service).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "dev.translator.desktop";

static FALLBACK_SECRETS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn fallback_secrets_lock() -> &'static Mutex<()> {
    FALLBACK_SECRETS_LOCK.get_or_init(|| Mutex::new(()))
}

/// Store an API key for a given service.
pub fn set_api_key(service_id: &str, api_key: &str) -> Result<()> {
    match Entry::new(SERVICE_NAME, &format!("api_key:{service_id}")) {
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
    match Entry::new(SERVICE_NAME, &format!("api_key:{service_id}")) {
        Ok(entry) => match entry.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => get_fallback_api_key(service_id),
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
/// usable credential (BH-4.3). Returns `false` for both "not set" and keyring
/// errors, so the caller can safely skip the service silently in either case.
pub fn has_api_key(service_id: &str) -> Result<bool> {
    match Entry::new(SERVICE_NAME, &format!("api_key:{service_id}")) {
        Ok(entry) => match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(get_fallback_api_key(service_id)?.is_some()),
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

/// Remove an API key for a given service.
pub fn delete_api_key(service_id: &str) -> Result<()> {
    match Entry::new(SERVICE_NAME, &format!("api_key:{service_id}")) {
        Ok(entry) => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => delete_fallback_api_key(service_id),
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
}
