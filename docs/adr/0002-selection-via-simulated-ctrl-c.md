# Obtain the Selection by simulating Ctrl+C and hijacking the clipboard

Windows has no clean, universal API for "read the text the user currently has selected." Our approach: back up the existing clipboard → send Ctrl+C to the foreground window → read the clipboard → restore the original contents. This is what mature tools such as Bob and PopClip all do, and its compatibility covers nearly every application.

## Considered Options

**UI Automation (TextPattern.GetSelection)** is the semantically "correct" approach and never touches the clipboard. It was rejected because measured coverage is insufficient — support across Chrome, Electron apps, and Office is uneven, and the overwhelming majority of cases would still fall back to the Ctrl+C branch. Maintaining two text-grabbing paths for a minority of cases is not worth it. This route can be layered in front as an **optimization** at any time later, without disturbing the current architecture.

## Consequences

- The clipboard is momentarily polluted and then restored. Rich text and images cannot be restored perfectly — **only the plain-text format is backed up and restored**, and we accept this known defect.
- There is a race between sending Ctrl+C and reading the clipboard. Poll for a change in the clipboard sequence number rather than sleeping for a fixed interval.
- Grabbing text inside a UAC-elevated window fails silently. When the grab comes back empty, handle it according to the Action's Input Source; it is not an error.
- The backed-up clipboard content is **discarded from memory immediately** once restoration completes; it does not linger in the process. A user's clipboard history should not gain an extra retained copy just because they use Beckon.
- The Popover offers **no "replace the original text"**. Automatic write-back means putting the result on the clipboard and simulating Ctrl+V, but there is no reliable signal for "paste finished," so the moment to restore can only be guessed. Grabbing text is an operation the user cannot see and therefore must be undone; write-back, by contrast, is not worth introducing that undebuggable race for a sliver of convenience. The only way a user takes the result away is by clicking "Copy" — that write was requested by the user, so it is not undone.
