---
status: accepted
---

# Store the API key in the OS credential store, never in plaintext on disk

**Generalised by [ADR-0013](0013-support-macos-alongside-windows.md):** the store is the login
Keychain on macOS and the Windows Credential Manager on Windows. `keyring` picks the backend per
target, the service/account pair is the same on both, and nothing above `secrets.rs` knows which
one it is talking to. Every property below holds on both: bound to the current user account,
useless if copied to another machine, and the three outcomes stay three outcomes.

The DeepSeek API key is stored in the Windows Credential Manager via the `keyring` crate (service name `Beckon`) and is written to no config file. There is no `secrets.toml` on disk.

On Windows, `keyring` is a wrapper over the Credential Manager (backed by DPAPI); the actual code is under ten lines and contains no `unsafe` — less work than hand-writing `CryptProtectData`. The credential is bound to the current Windows account and cannot be decrypted if copied to another machine.

## This decision was once reversed, and the reason is worth recording

The original decision was to store the key in plaintext in a separate `secrets.toml`, on the grounds that "this is single-machine personal use, so the convenience of hand-editing and backing up outweighs the marginal benefit of encryption."

What overturned it was not a security argument but the fact that **a later decision invalidated that premise**: the project subsequently committed to a full settings window with a "Test connection" button, so entering, changing, and verifying the key all happen in the UI — "hand-editing the config file in Notepad" will never actually happen in practice. Once the premise was gone, plaintext was pure risk with nothing left on the other side.

A reminder: the rationale behind an early decision can be quietly hollowed out by a later one, which makes it worth revisiting.

## Consequences

- Switching machines or reinstalling the OS means entering the key once more; copying over the config directory will not help. Acceptable.
- The key cannot be eyeballed in a text editor. The settings window must be able to echo it back (showing only the last four characters is suggested), or the user has no way to tell which key is stored.
- **The condition for "first run" is "no key readable from the Credential Manager"**, not the presence or absence of some file.
- A read failure (the credential was manually deleted by the user from Control Panel) must be distinguished from "the key is invalid": the former should guide the user through reconfiguration, the latter should report a bad key.
