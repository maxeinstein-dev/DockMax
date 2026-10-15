//! Storage of domain values on disk. One concrete implementation per value,
//! without storage traits (spec 0002, Q26); tests use temporary folders.

// Nothing outside these modules uses the widget database until the commands
// arrive; the integration task (I1) removes these `allow`s.
#[allow(dead_code)]
pub mod database;
#[allow(dead_code)]
pub mod notes_repo;
pub mod settings_file;
#[allow(dead_code)]
pub mod widgets_repo;
