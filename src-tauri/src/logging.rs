//! Logs of the Rust side and of both webviews, in one rotating file in the
//! app's log folder (`%LOCALAPPDATA%\<identifier>\logs` on Windows), plus
//! stdout in debug builds (spec 0002 §2.3 item 5).
//!
//! Messages name the capability affected and the kind of error only: never
//! a file path, settings content, argv or a command payload.
//!
//! Logging never stops the app. The plugin is registered without a logger,
//! only for its `log` command (the webviews' bridge); the logger is attached
//! in [`setup`], and when the log file can't be opened (locked during a
//! rotation, folder not writable...) it falls back to one without the file.

use log::LevelFilter;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_log::{
    Builder, Error, RotationStrategy, Target, TargetKind, TimezoneStrategy, WEBVIEW_TARGET,
};

/// Log file name, without the `.log` extension.
const FILE_NAME: &str = "dockmax";

/// Size at which the log file is rotated, in bytes.
const MAX_FILE_SIZE: u128 = 1024 * 1024;

/// Rotated files kept besides the active one. In plugin 2.10.1
/// `KeepSome(n)` keeps n rotated files plus the active one, so 4 + 1 = 5
/// files of up to 1 MiB: about 5 MB on disk.
const ARCHIVED_FILES: usize = 4;

/// Level of Dockyard's own messages, from Rust and from the webviews.
const APP_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};

/// Level of everything not in [`LEVELS`]: the dependencies (Tauri, wry,
/// tao...). Kept at `Warn`: their messages are not reviewed for paths or
/// payloads.
const DEPENDENCY_LEVEL: LevelFilter = LevelFilter::Warn;

/// Per-target levels: Dockyard's Rust code and the webviews.
const LEVELS: [(&str, LevelFilter); 2] = [
    (env!("CARGO_CRATE_NAME"), APP_LEVEL),
    (WEBVIEW_TARGET, APP_LEVEL),
];

/// Where log lines are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Output {
    /// The rotating file in the app's log folder.
    File,
    Stdout,
}

/// The outputs of the logger: the file, unless it could not be opened, and
/// stdout in debug builds (a release build has no console).
fn outputs(with_file: bool, debug: bool) -> Vec<Output> {
    let mut outputs = Vec::with_capacity(2);
    if with_file {
        outputs.push(Output::File);
    }
    if debug {
        outputs.push(Output::Stdout);
    }
    outputs
}

fn builder(outputs: &[Output]) -> Builder {
    let mut builder = Builder::new()
        .clear_targets()
        .max_file_size(MAX_FILE_SIZE)
        .rotation_strategy(RotationStrategy::KeepSome(ARCHIVED_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .level(DEPENDENCY_LEVEL);
    for (target, level) in LEVELS {
        builder = builder.level_for(target, level);
    }
    for output in outputs {
        builder = builder.target(Target::new(match output {
            Output::File => TargetKind::LogDir {
                file_name: Some(FILE_NAME.into()),
            },
            Output::Stdout => TargetKind::Stdout,
        }));
    }
    builder
}

/// The plugin, for its `log` command only: it attaches no logger, so it
/// can't fail the app's startup. [`setup`] attaches the logger.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new().skip_logger().build()
}

/// Attaches the logger, first thing in the app's setup. Never fails: without
/// the log file, the app logs to stdout in debug builds and nowhere in
/// release, and says so on the first line it can write.
pub fn setup<R: Runtime>(app: &AppHandle<R>) {
    let debug = cfg!(debug_assertions);
    let acquire = |with_file| {
        // The plugin `split` returns is the same as `plugin()`'s, already
        // registered: only the logger is kept.
        builder(&outputs(with_file, debug))
            .split(app)
            .map(|(_plugin, max_level, logger)| (max_level, logger))
    };
    let Some(((max_level, logger), issue)) = with_fallback(acquire(true), || acquire(false)) else {
        return;
    };
    // Fails only if a logger is already attached, which the app never does.
    if tauri_plugin_log::attach_logger(max_level, logger).is_err() {
        return;
    }
    if let Some(error) = issue {
        log::warn!(
            "logging: log file unavailable, logging without it: {}",
            failure_kind(&error)
        );
    }
}

/// `primary` when it worked; else `fallback`, with the error of `primary`;
/// `None` when both failed.
fn with_fallback<T, E>(
    primary: Result<T, E>,
    fallback: impl FnOnce() -> Result<T, E>,
) -> Option<(T, Option<E>)> {
    match primary {
        Ok(value) => Some((value, None)),
        Err(error) => fallback().ok().map(|value| (value, Some(error))),
    }
}

/// The kind of a logger failure, safe to log: the plugin's errors may carry
/// the log file path.
fn failure_kind(error: &Error) -> String {
    match error {
        Error::Io(error) => format!("{:?}", error.kind()),
        Error::Tauri(_) => "log folder unavailable".into(),
        Error::TimeFormat(_) | Error::InvalidFormatDescription(_) => "time format".into(),
        Error::LoggerNotInitialized => "logger not initialized".into(),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    const MIB: u128 = 1024 * 1024;

    #[test]
    fn rotation_keeps_about_5_mib_in_5_files() {
        assert_eq!(MAX_FILE_SIZE, MIB);
        // Plugin 2.10.1: `KeepSome(n)` keeps n rotated files besides the
        // active one.
        assert_eq!(ARCHIVED_FILES, 4);
        let files = ARCHIVED_FILES as u128 + 1;
        assert_eq!(files, 5);
        assert_eq!(files * MAX_FILE_SIZE, 5 * MIB);
    }

    #[test]
    fn dockyard_and_the_webviews_log_more_than_the_dependencies() {
        let app = if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };

        assert_eq!(LEVELS, [("dockyard_lib", app), ("webview", app)]);
        assert_eq!(DEPENDENCY_LEVEL, LevelFilter::Warn);
    }

    #[test]
    fn logs_go_to_the_file_and_to_stdout_only_in_debug() {
        assert_eq!(outputs(true, true), [Output::File, Output::Stdout]);
        assert_eq!(outputs(true, false), [Output::File]);
    }

    #[test]
    fn without_the_file_logs_go_to_stdout_in_debug_and_nowhere_in_release() {
        assert_eq!(outputs(false, true), [Output::Stdout]);
        assert_eq!(outputs(false, false), []);
    }

    #[test]
    fn a_logger_that_fails_is_replaced_by_the_fallback_and_reported() {
        let primary: Result<&str, &str> = Err("file locked");

        assert_eq!(
            with_fallback(primary, || Ok("console")),
            Some(("console", Some("file locked")))
        );
    }

    #[test]
    fn a_working_logger_is_kept_without_an_issue() {
        let fallback_used = std::cell::Cell::new(false);

        let chosen = with_fallback(Ok::<_, &str>("file"), || {
            fallback_used.set(true);
            Ok("console")
        });

        assert_eq!(chosen, Some(("file", None)));
        assert!(!fallback_used.get());
    }

    #[test]
    fn when_both_loggers_fail_the_app_goes_on_without_one() {
        assert_eq!(
            with_fallback(Err::<&str, _>("file"), || Err("console")),
            None
        );
    }

    #[test]
    fn failure_kind_never_contains_a_path() {
        let error = tauri_plugin_log::Error::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            r"C:\Users\someone\AppData\Local\dev.maxsueleinstein.dockmax\logs\dockmax.log",
        ));

        let kind = failure_kind(&error);

        assert_eq!(kind, "PermissionDenied");
        assert!(!kind.contains('\\') && !kind.contains('/'), "{kind}");
        assert_eq!(
            failure_kind(&tauri_plugin_log::Error::LoggerNotInitialized),
            "logger not initialized"
        );
    }
}
