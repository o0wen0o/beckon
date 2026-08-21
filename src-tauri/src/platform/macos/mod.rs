//! AppKit and CoreGraphics implementations. Nothing outside this directory
//! uses `objc2`, `objc2-app-kit` or `core-graphics` (ADR-0001, ADR-0013).

pub mod focus;
pub mod permission;
pub mod selection;
