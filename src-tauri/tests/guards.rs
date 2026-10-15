//! Guard tests: pin the IPC surface (capabilities) and the CSP so that any
//! widening shows up as a visible change to this file (ADR 0006, spec 0002
//! §2.3 item 1).
//!
//! Permissions are compared with set semantics plus an explicit length check:
//! order inside a capability has no meaning to Tauri, so reordering must not
//! fail, but an added, removed or duplicated entry must.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Expected capabilities: (file name, identifier, windows, permissions).
/// Changing a capability requires changing this table.
const EXPECTED_CAPABILITIES: &[(&str, &str, &[&str], &[&str])] = &[
    (
        "dock.json",
        "dock",
        &["dock"],
        &[
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            "allow-toggle-panel",
            "allow-set-dock-layout",
            "allow-get-settings",
            "log:allow-log",
        ],
    ),
    (
        "panel.json",
        "panel",
        &["panel"],
        &[
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            "allow-hide-panel",
            "allow-panel-ready",
            "allow-get-settings",
            "allow-update-settings",
            "log:allow-log",
        ],
    ),
];

/// Top-level keys a capability file may have. Anything else (`remote`,
/// `platforms`, `webviews`, `local`...) changes the scope of the capability.
const ALLOWED_CAPABILITY_KEYS: &[&str] = &[
    "$schema",
    "identifier",
    "description",
    "windows",
    "permissions",
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()))
}

fn string_list(value: &Value, file: &str, key: &str) -> Vec<String> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{file}: `{key}` must be an array"));
    array
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| {
                    panic!("{file}: every entry of `{key}` must be a string, found {item}")
                })
                .to_owned()
        })
        .collect()
}

fn as_set<S: AsRef<str>>(items: &[S]) -> BTreeSet<String> {
    items.iter().map(|s| s.as_ref().to_owned()).collect()
}

#[test]
fn capability_files_are_exactly_the_expected_ones() {
    let dir = manifest_dir().join("capabilities");
    let found: BTreeSet<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("cannot read directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let expected: BTreeSet<String> = EXPECTED_CAPABILITIES
        .iter()
        .map(|(file, ..)| (*file).to_owned())
        .collect();

    let unexpected: Vec<_> = found.difference(&expected).collect();
    let missing: Vec<_> = expected.difference(&found).collect();
    assert!(
        unexpected.is_empty() && missing.is_empty(),
        "capabilities/ must contain exactly {expected:?}; unexpected files: {unexpected:?}, missing files: {missing:?}"
    );
}

#[test]
fn capability_contents_are_pinned() {
    let dir = manifest_dir().join("capabilities");
    for (file, identifier, windows, permissions) in EXPECTED_CAPABILITIES {
        let json = read_json(&dir.join(file));
        let object = json
            .as_object()
            .unwrap_or_else(|| panic!("{file}: top level must be a JSON object"));

        for key in object.keys() {
            assert!(
                ALLOWED_CAPABILITY_KEYS.contains(&key.as_str()),
                "{file}: unexpected top-level key `{key}` (allowed: {ALLOWED_CAPABILITY_KEYS:?})"
            );
        }

        assert_eq!(
            json.get("identifier").and_then(Value::as_str),
            Some(*identifier),
            "{file}: `identifier` must be {identifier:?}"
        );

        let actual_windows = string_list(&json, file, "windows");
        assert_eq!(
            actual_windows,
            windows.to_vec(),
            "{file}: `windows` must be exactly {windows:?}"
        );

        let actual = string_list(&json, file, "permissions");
        let actual_set = as_set(&actual);
        let expected_set = as_set(permissions);
        let unexpected: Vec<_> = actual_set.difference(&expected_set).collect();
        let missing: Vec<_> = expected_set.difference(&actual_set).collect();
        assert!(
            unexpected.is_empty() && missing.is_empty(),
            "{file}: permissions changed; unexpected: {unexpected:?}, missing: {missing:?}. \
             Update EXPECTED_CAPABILITIES in tests/guards.rs only if the change is intended (ADR 0006)."
        );
        assert_eq!(
            actual.len(),
            permissions.len(),
            "{file}: `permissions` has duplicate entries: {actual:?}"
        );
    }
}

#[test]
fn no_blanket_default_permissions() {
    let dir = manifest_dir().join("capabilities");
    let entries =
        fs::read_dir(&dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("cannot read directory entry").path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue; // non-JSON files are rejected by the file-set guard
        }
        let json = read_json(&path);
        for permission in string_list(&json, &file, "permissions") {
            assert!(
                permission != "core:default" && !permission.ends_with(":default"),
                "{file}: blanket default permission `{permission}` is banned (ADR 0006); grant the specific permission instead"
            );
        }
    }
}

fn check_csp(key: &str, value: Option<&Value>) {
    let csp = value.and_then(Value::as_str).unwrap_or_else(|| {
        panic!("tauri.conf.json: `app.security.{key}` must be a non-null string, found {value:?}")
    });

    for banned in ["unsafe-eval", "unsafe-inline"] {
        assert!(
            !csp.contains(banned),
            "tauri.conf.json: `app.security.{key}` must not contain `{banned}`: {csp}"
        );
    }

    for directive in csp.split(';') {
        for source in directive.split_whitespace().skip(1) {
            assert!(
                !source.contains('*'),
                "tauri.conf.json: `app.security.{key}` has wildcard source `{source}` in `{}`",
                directive.trim()
            );
        }
    }

    let has_default_self = csp.split(';').any(|directive| {
        let mut parts = directive.split_whitespace();
        parts.next() == Some("default-src") && parts.any(|source| source == "'self'")
    });
    assert!(
        has_default_self,
        "tauri.conf.json: `app.security.{key}` must include `default-src 'self'`: {csp}"
    );
}

#[test]
fn csp_is_strict() {
    let conf = read_json(&manifest_dir().join("tauri.conf.json"));
    let security = conf
        .pointer("/app/security")
        .unwrap_or_else(|| panic!("tauri.conf.json: `app.security` is missing"));

    check_csp("csp", security.get("csp"));
    if let Some(dev) = security.get("devCsp") {
        check_csp("devCsp", Some(dev));
    }
}

#[test]
fn no_inline_capabilities_in_config() {
    let conf = read_json(&manifest_dir().join("tauri.conf.json"));
    assert!(
        conf.pointer("/app/security/capabilities").is_none(),
        "tauri.conf.json: `app.security.capabilities` must not be set; capabilities live only in capabilities/*.json"
    );
}

#[test]
fn log_plugin_cannot_fail_startup() {
    // The plugin is registered for its `log` command only; the logger, which
    // opens the log file and can fail, is attached in `logging::setup` with a
    // fallback (ADR 0007). Building the plugin with its logger again would let
    // a locked log file abort the app.
    let source = std::fs::read_to_string(manifest_dir().join("src").join("logging.rs"))
        .expect("src/logging.rs must exist");
    let body = source
        .split("pub fn plugin")
        .nth(1)
        .and_then(|rest| {
            rest.split(
                "
}
",
            )
            .next()
        })
        .unwrap_or_else(|| panic!("src/logging.rs: `pub fn plugin` not found"));
    assert!(
        body.contains(".skip_logger()"),
        "src/logging.rs: `logging::plugin` must build the log plugin with `.skip_logger()`; the logger is attached in `logging::setup`"
    );
}

#[test]
fn logger_and_settings_start_before_the_windows() {
    // Tauri creates the windows of tauri.conf.json before the app's `setup`
    // runs, but initializes plugins earlier, in `Builder::build`. Loading the
    // settings (or attaching the logger) in `setup` let the dock call
    // `get_settings` before the state existed, so it never got its theme.
    let source = std::fs::read_to_string(manifest_dir().join("src").join("lib.rs"))
        .expect("src/lib.rs must exist");
    let app_setup = source
        .split(".setup(|app|")
        .nth(1)
        .and_then(|rest| rest.split(".invoke_handler(").next())
        .unwrap_or_else(|| panic!("src/lib.rs: the app's `.setup(|app| ...)` was not found"));
    for call in ["logging::setup", "commands::settings::setup"] {
        assert!(
            !app_setup.contains(call),
            "src/lib.rs: `{call}` must run in the `startup` plugin, not in the app's `setup` (the windows already exist there)"
        );
    }
    assert!(
        source.contains(".plugin(startup())"),
        "src/lib.rs: the `startup` plugin, which attaches the logger and loads the settings, must be registered"
    );
}
