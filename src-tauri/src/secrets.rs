//! One API key **per provider**, in the OS credential store (ADR-0005,
//! ADR-0013, ADR-0021): the Windows Credential Manager, or the login Keychain on
//! macOS. `keyring` picks the backend per target and the service/account pair is
//! the same on both, so nothing above this module knows which store it is
//! talking to.
//!
//! The account is `provider:{id}` — one credential per row of `[[api.providers]]`,
//! because two endpoints can be live at once and a key is only ever valid at the
//! host it was issued for. The id is a [`Provider::id`](crate::config::Provider),
//! so renaming a row's *label* keeps its key and changing its `id` deliberately
//! does not.
//!
//! The three outcomes below must stay distinguishable all the way to the UI:
//! "no credential" guides the user through reconfiguration, a read error points
//! at the credential store, and neither may ever be shown as "your key is
//! invalid".
//!
//! Since ADR-0021 there is a fourth reading, and it is *not* an outcome of this
//! module: nothing stored for a **local** endpoint is a working setup rather than
//! a fault, because a local server wants no `Authorization` header at all. This
//! module reports what is stored; `Provider::is_local` decides whether its
//! absence matters.

use keyring::Entry;
use serde::Serialize;

pub const SERVICE: &str = "Beckon";
/// The single account every version before the provider table wrote to. Read
/// once, on the way to `provider:{default}`; see [`migrate_legacy`].
pub const LEGACY_ACCOUNT: &str = "api-key";

/// The credential account for one provider row.
pub fn account(provider_id: &str) -> String {
    format!("provider:{provider_id}")
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeyStatus {
    /// A key is stored. Only the tail is echoed back — ADR-0005 asks for enough
    /// to identify the key, not enough to read it off the screen.
    Present { last4: String },
    /// Nothing stored: this is the first-run condition (README, ADR-0005).
    NoCredential,
    /// The credential store itself failed.
    ReadError { message: String },
}

fn entry(account: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, account).map_err(|e| e.to_string())
}

pub fn status(provider_id: &str) -> KeyStatus {
    match read(provider_id) {
        Ok(Some(key)) => KeyStatus::Present { last4: last4(&key) },
        Ok(None) => KeyStatus::NoCredential,
        Err(message) => KeyStatus::ReadError { message },
    }
}

/// `Ok(None)` means "no credential"; `Err` means the store failed. Conflating
/// the two is the mistake ADR-0005 calls out.
pub fn read(provider_id: &str) -> Result<Option<String>, String> {
    read_account(&account(provider_id))
}

fn read_account(account: &str) -> Result<Option<String>, String> {
    match entry(account)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

pub fn write(provider_id: &str, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("the API key must not be empty".to_string());
    }
    entry(&account(provider_id))?
        .set_password(key)
        .map_err(|e| e.to_string())
}

pub fn delete(provider_id: &str) -> Result<(), String> {
    match entry(&account(provider_id))?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

/// Copy the pre-provider credential onto the default provider's account, once
/// (ADR-0021).
///
/// Runs at startup, before the first-run check, so an existing install does not
/// come back reporting no key and yanking the user to Settings.
///
/// Two deliberate choices:
///
/// - It never overwrites. A key already at `provider:{id}` is the newer fact.
/// - The legacy entry is **left in place**, not moved. Deleting somebody's
///   credential is not a migration's business, and a downgrade to a build
///   without the provider table then still works. It costs one dead entry in a
///   store the user can see and clear themselves.
///
/// A store that cannot be read is not an error worth surfacing here: the very
/// next thing that happens is a per-provider status read, which reports it in
/// the terms ADR-0005 asks for.
pub fn migrate_legacy(provider_id: &str) -> bool {
    if !matches!(read(provider_id), Ok(None)) {
        return false;
    }
    match read_account(LEGACY_ACCOUNT) {
        Ok(Some(key)) => write(provider_id, &key).is_ok(),
        _ => false,
    }
}

fn last4(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let tail = chars.len().min(4);
    chars[chars.len() - tail..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echoes_only_the_tail() {
        assert_eq!(last4("sk-abcdef1234"), "1234");
        assert_eq!(last4("abc"), "abc");
        assert_eq!(last4(""), "");
    }

    #[test]
    fn statuses_serialize_with_a_discriminant() {
        let json = serde_json::to_string(&KeyStatus::NoCredential).unwrap();
        assert_eq!(json, r#"{"kind":"no-credential"}"#);
        let json = serde_json::to_string(&KeyStatus::Present {
            last4: "1234".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"present","last4":"1234"}"#);
    }

    /// The account is derived from the row's id, and it is namespaced: a row a
    /// user calls `api-key` must not collide with the pre-provider account the
    /// migration reads (ADR-0021).
    #[test]
    fn accounts_are_namespaced_per_provider() {
        assert_eq!(account("deepseek"), "provider:deepseek");
        assert_ne!(account(LEGACY_ACCOUNT), LEGACY_ACCOUNT);
    }
}
