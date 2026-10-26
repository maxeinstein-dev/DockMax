//! Storage of domain values on disk. One concrete implementation per value,
//! without storage traits (spec 0002, Q26); tests use temporary folders.

pub mod apps_repo;
pub mod database;
pub mod icon_packs;
pub mod notes_repo;
pub mod settings_file;
pub mod widgets_repo;
