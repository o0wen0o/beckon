//! Win32 implementations. Nothing outside this directory uses the `windows`
//! crate (ADR-0001).

mod clipboard;
pub mod focus;
pub mod permission;
pub mod selection;
pub mod snip;
