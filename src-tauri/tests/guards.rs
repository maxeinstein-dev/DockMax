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
        "panel.json",
        "panel",
        &["panel"],
        &[
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            "allow-hide-panel",
            "allow-panel-ready",
            "allow-panel-mounted",
            "allow-get-settings",
            "allow-update-settings",
            "allow-list-widgets",
            "allow-add-widget",
            "allow-remove-widget",
            "allow-move-widget",
            "allow-update-widget",
            "allow-get-note",
            "allow-save-note",
            "allow-dismiss-storage-notice",
            // The tray panel lists the notification area and clicks its
            // icons (ADR 0009, Fase E); a click names an icon the list gave.
            "allow-list-tray",
            "allow-tray-click",
            // The apps list in Configurações: the catalog to choose from,
            // and pin, unpin and reorder (spec 0006).
            "allow-list-apps",
            "allow-list-catalog",
            "allow-app-icon",
            "allow-pin-app",
            "allow-unpin-app",
            "allow-move-app",
            // The menu of a pinned program (spec 0006 §6) shows in the
            // panel: its Jump List, opening it, and showing its file. Each
            // names a pinned item; an entry is run by the key of the menu
            // last shown, never by a path.
            "allow-launch-app",
            "allow-app-menu",
            "allow-run-app-entry",
            "allow-reveal-app",
            "allow-list-open-apps",
            "allow-launch-open",
            "allow-close-app",
            "allow-close-open",
            "allow-pin-open",
            "allow-list-icon-packs",
            "allow-reload-icon-packs",
            "allow-open-icon-packs-folder",
            "log:allow-log",
        ],
    ),
    (
        "settings.json",
        "settings",
        &["settings"],
        &[
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            // Esc closes the window, as it closes a panel.
            "core:window:allow-close",
            // The Configurações app (spec 0011 H1): the settings, the dock's
            // items and apps, the icon packs. No panel, menu or tray command.
            "allow-get-settings",
            "allow-update-settings",
            "allow-list-widgets",
            "allow-add-widget",
            "allow-remove-widget",
            "allow-move-widget",
            "allow-dismiss-storage-notice",
            "allow-list-apps",
            "allow-list-catalog",
            "allow-app-icon",
            "allow-pin-app",
            "allow-unpin-app",
            "allow-move-app",
            "allow-list-icon-packs",
            "allow-reload-icon-packs",
            "allow-open-icon-packs-folder",
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

/// Whether `c` can be part of an identifier (so an `r` after it is not the
/// start of a raw string).
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `source` without its `//` and `/* */` comments (nested ones included),
/// so the guards that look for a call in the code are not satisfied by a
/// commented-out one. String, raw string and character literals are copied
/// as they are: the `//` of `"https://..."` is not a comment. A block comment
/// becomes a space, so the tokens around it stay apart.
fn strip_comments(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while let Some(c) = at(i) {
        match (c, at(i + 1)) {
            ('/', Some('/')) => {
                while at(i).is_some_and(|c| c != '\n') {
                    i += 1;
                }
            }
            ('/', Some('*')) => {
                let mut depth = 0usize;
                while let Some(c) = at(i) {
                    if c == '/' && at(i + 1) == Some('*') {
                        depth += 1;
                        i += 2;
                    } else if c == '*' && at(i + 1) == Some('/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                out.push(' ');
            }
            ('"', _) => {
                out.push('"');
                i += 1;
                while let Some(c) = at(i) {
                    out.push(c);
                    i += 1;
                    if c == '\\' {
                        if let Some(escaped) = at(i) {
                            out.push(escaped);
                            i += 1;
                        }
                    } else if c == '"' {
                        break;
                    }
                }
            }
            ('r', Some('"' | '#'))
                if i == 0 || !is_ident_char(chars[i - 1]) || chars[i - 1] == 'b' =>
            {
                let hashes = chars[i + 1..].iter().take_while(|&&c| c == '#').count();
                if at(i + 1 + hashes) != Some('"') {
                    // `r#ident`, a raw identifier.
                    out.push(c);
                    i += 1;
                    continue;
                }
                let start = i;
                i += hashes + 2;
                while let Some(c) = at(i) {
                    i += 1;
                    if c == '"' && chars[i..].iter().take_while(|&&c| c == '#').count() >= hashes {
                        i += hashes;
                        break;
                    }
                }
                out.extend(&chars[start..i.min(chars.len())]);
            }
            ('\'', Some('\\')) => {
                // An escaped character literal: `'\''`, `'\n'`, `'\u{..}'`.
                let start = i;
                i += 3;
                while at(i).is_some_and(|c| c != '\'') {
                    i += 1;
                }
                i += 1;
                out.extend(&chars[start..i.min(chars.len())]);
            }
            ('\'', Some(_)) if at(i + 2) == Some('\'') => {
                // A plain character literal, such as `'"'`.
                out.extend(&chars[i..i + 3]);
                i += 3;
            }
            _ => {
                // Code, or the `'` of a lifetime.
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The Rust source at `relative` (from the crate root), without comments.
fn rust_code(relative: &str) -> String {
    let path = manifest_dir().join(relative);
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    strip_comments(&source)
}

#[test]
fn strip_comments_keeps_code_and_literals() {
    let source = r##"let a = 1; // gone::setup(app);
/* gone /* nested */ still gone */call(b);
let url = "https://x/*y*/"; let q = '"'; let s = '\''; fn f<'a>(x: &'a str) {}
let raw = r#"// kept"#; /// doc gone
"##;
    let expected = "let a = 1; \n call(b);
let url = \"https://x/*y*/\"; let q = '\"'; let s = '\\''; fn f<'a>(x: &'a str) {}
let raw = r#\"// kept\"#; \n";
    assert_eq!(strip_comments(source), expected);
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
    let source = rust_code("src/logging.rs");
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
fn logger_settings_and_widgets_start_before_the_windows() {
    // Tauri creates the windows of tauri.conf.json before the app's `setup`
    // runs, but initializes plugins earlier, in `Builder::build`. Loading the
    // settings (or attaching the logger) in `setup` let the dock call
    // `get_settings` before the state existed, so it never got its theme; the
    // widget database has the same need for `list_widgets`.
    // Without comments: a commented-out call must not count.
    let source = rust_code("src/lib.rs");
    let app_setup = source
        .split(".setup(|app|")
        .nth(1)
        .and_then(|rest| rest.split(".invoke_handler(").next())
        .unwrap_or_else(|| panic!("src/lib.rs: the app's `.setup(|app| ...)` was not found"));
    // In this order: the logger first, so the others can log; the settings
    // before the widgets, so a broken database never keeps them from loading.
    let calls = [
        "logging::setup",
        "commands::settings::setup",
        "commands::widgets::setup",
    ];
    for call in calls {
        assert!(
            !app_setup.contains(call),
            "src/lib.rs: `{call}` must run in the `startup` plugin, not in the app's `setup` (the windows already exist there)"
        );
    }
    let startup = source
        .split("fn startup()")
        .nth(1)
        .and_then(|rest| rest.split(".build()").next())
        .unwrap_or_else(|| panic!("src/lib.rs: `fn startup()` was not found"));
    let positions: Vec<usize> = calls
        .iter()
        .map(|call| {
            startup
                .find(call)
                .unwrap_or_else(|| panic!("src/lib.rs: `startup` must call `{call}`"))
        })
        .collect();
    assert!(
        positions.is_sorted(),
        "src/lib.rs: `startup` must call {calls:?} in this order"
    );
    assert!(
        source.contains(".plugin(startup())"),
        "src/lib.rs: the `startup` plugin, which attaches the logger, loads the settings and opens the widget database, must be registered"
    );
}

/// The `{ ... }` block that follows the first `marker` of `code` not
/// followed by an identifier character (`RunEvent::Exit`, not
/// `RunEvent::ExitRequested`), without its braces.
fn block_after<'a>(code: &'a str, marker: &str, file: &str) -> &'a str {
    let start = code
        .match_indices(marker)
        .map(|(index, _)| index + marker.len())
        .find(|&end| !code[end..].starts_with(is_ident_char))
        .unwrap_or_else(|| panic!("{file}: `{marker}` not found"));
    let rest = &code[start..];
    let open = rest
        .find('{')
        .unwrap_or_else(|| panic!("{file}: no block after `{marker}`"));
    let mut depth = 0usize;
    for (index, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[open + 1..open + index];
                }
            }
            _ => {}
        }
    }
    panic!("{file}: the block after `{marker}` is not closed");
}

#[test]
fn exit_checkpoints_the_widget_database() {
    // The process ends without closing the widget database: without the
    // checkpoint on `RunEvent::Exit`, the last changes stay only in the
    // `-wal` (spec 0003 §2.4). Without comments: a commented-out call must
    // not count.
    let source = rust_code("src/lib.rs");
    let exit = block_after(&source, "RunEvent::Exit", "src/lib.rs");
    let checkpoint = exit.find(".checkpoint()").unwrap_or_else(|| {
        panic!("src/lib.rs: `RunEvent::Exit` must call `.checkpoint()` on the widget storage")
    });
    assert!(
        exit[..checkpoint].contains("persistence::database::Storage"),
        "src/lib.rs: the `.checkpoint()` on `RunEvent::Exit` must be the widget storage's (`persistence::database::Storage`)"
    );
}

/// The comma-separated items between `open` and the next `]` of `code`,
/// which must appear exactly once.
fn bracket_items<'a>(code: &'a str, open: &str, file: &str) -> Vec<&'a str> {
    let found = code.matches(open).count();
    assert_eq!(
        found, 1,
        "{file}: expected exactly one `{open}`, found {found}"
    );
    code.split(open)
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap_or_else(|| panic!("{file}: `{open}` is not closed by `]`"))
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect()
}

#[test]
fn every_declared_command_is_registered_and_vice_versa() {
    // `build.rs` declares the commands that get an `allow-<command>`
    // permission; `generate_handler!` in `lib.rs` is what the app answers.
    // A command only on the first list has a permission and no handler; one
    // only on the second has a handler no capability can grant, so it fails
    // at runtime instead of here.
    let manifest = rust_code("build.rs");
    let declared: Vec<&str> = bracket_items(&manifest, ".commands(&[", "build.rs")
        .into_iter()
        .map(|item| {
            item.strip_prefix('"')
                .and_then(|item| item.strip_suffix('"'))
                .unwrap_or_else(|| {
                    panic!("build.rs: every command in `AppManifest::commands` must be a string literal, found `{item}`")
                })
        })
        .collect();
    let lib = rust_code("src/lib.rs");
    let registered: Vec<&str> = bracket_items(&lib, "generate_handler![", "src/lib.rs")
        .into_iter()
        .map(|path| path.rsplit("::").next().unwrap_or(path).trim())
        .collect();

    let declared_set = as_set(&declared);
    let registered_set = as_set(&registered);
    let only_declared: Vec<_> = declared_set.difference(&registered_set).collect();
    let only_registered: Vec<_> = registered_set.difference(&declared_set).collect();
    assert!(
        only_declared.is_empty() && only_registered.is_empty(),
        "the commands of `AppManifest` (build.rs) and `generate_handler!` (src/lib.rs) differ; \
         only in build.rs: {only_declared:?}, only in lib.rs: {only_registered:?}"
    );
    assert_eq!(
        declared.len(),
        declared_set.len(),
        "build.rs: `AppManifest::commands` has duplicate entries: {declared:?}"
    );
    assert_eq!(
        registered.len(),
        registered_set.len(),
        "src/lib.rs: `generate_handler!` has duplicate entries: {registered:?}"
    );
}

/// The Tauri config files Tauri can read from the crate root. Any other one
/// (`tauri.windows.conf.json`, `Tauri.toml`...) is merged into every build of
/// that platform, past the CSP and capability guards above.
const EXPECTED_TAURI_CONFIGS: &[&str] = &["tauri.conf.json", "tauri.e2e.conf.json"];

#[test]
fn tauri_config_files_are_exactly_the_expected_ones() {
    let dir = manifest_dir();
    let found: BTreeSet<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("cannot read directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| {
            let lower = name.to_lowercase();
            lower.starts_with("tauri")
                && [".json", ".json5", ".toml"]
                    .iter()
                    .any(|ext| lower.ends_with(ext))
        })
        .collect();
    assert_eq!(
        found,
        as_set(EXPECTED_TAURI_CONFIGS),
        "src-tauri/ must hold exactly these Tauri config files"
    );
}

#[test]
fn e2e_config_only_renames_the_app() {
    // `pnpm test:e2e:build` merges this file over tauri.conf.json. It may
    // only give the test build its own identifier and binary name: a CSP, a
    // capability or a window here would make the tested app differ from the
    // released one without tripping any other guard.
    let file = "tauri.e2e.conf.json";
    let json = read_json(&manifest_dir().join(file));
    let object = json
        .as_object()
        .unwrap_or_else(|| panic!("{file}: top level must be a JSON object"));
    let keys: BTreeSet<String> = object.keys().cloned().collect();
    assert_eq!(
        keys,
        as_set(&["$schema", "identifier", "mainBinaryName"]),
        "{file}: must set exactly `$schema`, `identifier` and `mainBinaryName`"
    );

    let base = read_json(&manifest_dir().join("tauri.conf.json"));
    let base_identifier = base
        .get("identifier")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("tauri.conf.json: `identifier` must be a string"));
    let identifier = json.get("identifier").and_then(Value::as_str);
    assert_eq!(
        identifier,
        Some(format!("{base_identifier}.e2e").as_str()),
        "{file}: `identifier` must be the app's own followed by `.e2e` (src/e2e.rs refuses any other)"
    );
    let binary = json.get("mainBinaryName").and_then(Value::as_str);
    assert!(
        binary.is_some_and(|name| !name.is_empty()),
        "{file}: `mainBinaryName` must be a non-empty string, found {binary:?}"
    );
}

#[test]
fn the_app_builder_registers_its_state_through_with_state() {
    // `tests/managed_state.rs` proves `with_state` manages the state before
    // any setup; this keeps `run` from skipping it, and keeps the state from
    // being managed anywhere else (after the plugins that read it).
    let source = rust_code("src/lib.rs");
    let run = source
        .split("pub fn run()")
        .nth(1)
        .unwrap_or_else(|| panic!("src/lib.rs: `pub fn run()` was not found"));
    assert!(
        run.contains("with_state(builder)"),
        "src/lib.rs: `run` must register the state with `with_state(builder)`"
    );
    assert!(
        !run.contains(".manage("),
        "src/lib.rs: `run` must not `.manage(...)` by itself; add it to `with_state`"
    );
    let with_state = block_after(&source, "fn with_state", "src/lib.rs");
    for state in ["PanelState"] {
        assert!(
            with_state.contains(state),
            "src/lib.rs: `with_state` must manage the {state} state"
        );
    }
}
