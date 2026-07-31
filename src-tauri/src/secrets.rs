//! The API key, in the Windows Credential Manager (ADR-0005).
//!
//! The three outcomes below must stay distinguishable all the way to the UI:
//! "no credential" guides the user through reconfiguration, a read error points
//! at the Credential Manager, and neither may ever be shown as "your key is
//! invalid".

use keyring::Entry;
use serde::Serialize;

pub const SERVICE: &str = "Beckon";
pub const ACCOUNT: &str = "api-key";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeyStatus {
    /// A key is stored. Only the tail is echoed back — ADR-0005 asks for enough
    /// to identify the key, not enough to read it off the screen.
    Present { last4: String },
    /// Nothing stored: this is the first-run condition (README, ADR-0005).
    NoCredential,
    /// The Credential Manager itself failed.
    ReadError { message: String },
}

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())
}

pub fn status() -> KeyStatus {
    match read() {
        Ok(Some(key)) => KeyStatus::Present { last4: last4(&key) },
        Ok(None) => KeyStatus::NoCredential,
        Err(message) => KeyStatus::ReadError { message },
    }
}

/// `Ok(None)` means "no credential"; `Err` means the store failed. Conflating
/// the two is the mistake ADR-0005 calls out.
pub fn read() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

pub fn write(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("the API key must not be empty".to_string());
    }
    entry()?.set_password(key).map_err(|e| e.to_string())
}

pub fn delete() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
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
}
