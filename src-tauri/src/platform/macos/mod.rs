//! AppKit and CoreGraphics implementations. Nothing outside this directory
//! uses `objc2-app-kit`, `objc2-foundation` or `core-graphics`
//! (ADR-0001, ADR-0013).

pub mod focus;
pub mod permission;
pub mod selection;
pub mod snip;
