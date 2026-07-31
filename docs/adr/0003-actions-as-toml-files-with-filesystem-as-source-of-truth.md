# Store Actions as one TOML file each, with the filesystem as the single source of truth

Every Action is a TOML file under `%APPDATA%\<app>\actions\`. The settings window is an **editor** of those files, not their owner: saving writes to disk immediately, while a file watcher observes external changes and reloads automatically.

TOML was chosen over JSON because the bulk of an Action is a multi-line prompt — TOML's multi-line strings can be written by hand directly, whereas JSON requires escaping `\n`, which is unreadable when editing by hand. One file per Action was chosen over a single `actions.json` so that an individual Action can be copied, shared, or put under version control on its own, and so that corrupting one file does not destroy the entire configuration.

## Consequences

- The settings window **must not** hold authoritative in-memory state. Edits in the UI must land on disk immediately, and external changes must be able to refresh the UI in return; otherwise you get "my hand-edit was silently overwritten," the most infuriating kind of data loss.
- The file watcher needs debouncing and must tolerate the temporary files and atomic replacements editors use when saving (writing a temp file then renaming triggers delete + create rather than modify).
- An Action that fails to parse as TOML should be **skipped and flagged red in the UI**, not cause the whole load to fail.
- Categories can later be implemented with subdirectories, with no change to the file format — a side benefit of choosing a directory structure over a single file.
