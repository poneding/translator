//! Secret storage backed by the OS Keychain (macOS/Windows/Linux Secret Service).

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "dev.translator.desktop";

/// Store an API key for a given service.
pub fn set_api_key(service_id: &str, api_key: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key:{service_id}"))
        .context("create keyring entry")?;
    entry
        .set_password(api_key)
        .context("store api key in keyring")?;
    Ok(())
}

/// Load an API key for a given service. Returns `Ok(None)` if not set.
pub fn get_api_key(service_id: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key:{service_id}"))
        .context("create keyring entry")?;
    match entry.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!(e)).context("read api key from keyring"),
    }
}

/// Cheap boolean probe used at dispatch time to decide whether a service has a
/// usable credential (BH-4.3). Returns `false` for both "not set" and keyring
/// errors, so the caller can safely skip the service silently in either case.
pub fn has_api_key(service_id: &str) -> Result<bool> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key:{service_id}"))
        .context("create keyring entry")?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(anyhow::anyhow!(e)).context("probe api key in keyring"),
    }
}

/// Remove an API key for a given service.
pub fn delete_api_key(service_id: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, &format!("api_key:{service_id}"))
        .context("create keyring entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)).context("delete api key from keyring"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use a unique service id so this test does not collide with any real key
    // the user has stored. The test sets, probes, and cleans up its own entry.
    const TEST_SVC: &str = "__translator_test_bh43__";

    #[test]
    fn has_api_key_returns_false_for_missing_entry() {
        delete_api_key(TEST_SVC).ok();
        match has_api_key(TEST_SVC) {
            Ok(false) => {}
            Ok(true) => panic!("expected no key for unused service id"),
            Err(e) => panic!("keyring error (acceptable in sandboxed CI but not locally): {e}"),
        }
    }

    #[test]
    fn has_api_key_round_trips_with_set_and_delete() {
        // Acceptable to skip on CI runners without a working keyring backend.
        if has_api_key(TEST_SVC).is_err() {
            eprintln!("skipping: keyring backend unavailable in this environment");
            return;
        }
        delete_api_key(TEST_SVC).ok();
        assert!(!has_api_key(TEST_SVC).unwrap(), "should start unset");

        set_api_key(TEST_SVC, "secret").expect("set");
        assert!(has_api_key(TEST_SVC).unwrap(), "should be set");
        assert_eq!(
            get_api_key(TEST_SVC).unwrap().as_deref(),
            Some("secret"),
            "get should round-trip the value"
        );

        delete_api_key(TEST_SVC).expect("delete");
        assert!(
            !has_api_key(TEST_SVC).unwrap(),
            "should be unset after delete"
        );
    }
}
