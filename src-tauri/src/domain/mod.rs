//! Pure application rules: types, defaults and validation. Nothing here
//! touches the file system, Tauri or Windows.

pub mod settings;
// Nothing outside the module uses the widget contract until the commands
// arrive; the integration task (I1) removes this.
#[allow(dead_code)]
pub mod widgets;
