//! Storage of domain values on disk. One concrete implementation per value,
//! without storage traits (spec 0002, Q26); tests use temporary folders.

pub mod settings_file;
