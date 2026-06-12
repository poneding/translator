//! Placeholder icon directory.
//!
//! Real icons must be generated before the first `cargo tauri build`.
//! The canonical editable source is `app-icon-source.svg`. Render it to a
//! 1024x1024 transparent PNG at `app-icon-source.png`, then run:
//!
//! ```bash
//! cd crates/app
//! cargo tauri icon icons/app-icon-source.png
//! ```
//!
//! This will populate `32x32.png`, `128x128.png`, `128x128@2x.png`,
//! `icon.icns`, and `icon.ico` from the source PNG.
