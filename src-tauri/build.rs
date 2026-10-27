fn main() {
    // Integration test executables only (never the app): the mock app links
    // the WebView runtime, which imports `TaskDialogIndirect` from Common
    // Controls v6, and a test exe has no manifest asking for it ("entry point
    // not found" before the first test). The app's own manifest is embedded
    // by `tauri_build` below.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
    // Declaring the app commands makes each one require an explicit
    // permission (`allow-<command>`) in a capability.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "hide_panel",
            "panel_ready",
            "panel_mounted",
            "get_settings",
            "update_settings",
            "list_widgets",
            "add_widget",
            "remove_widget",
            "move_widget",
            "update_widget",
            "get_note",
            "save_note",
            "dismiss_storage_notice",
            "list_tray",
            "tray_click",
            "tray_to_dock",
            "list_apps",
            "list_catalog",
            "app_icon",
            "pin_app",
            "unpin_app",
            "move_app",
            "place_app",
            "launch_app",
            "list_open_apps",
            "activate_app",
            "activate_open",
            "launch_open",
            "open_badge",
            "close_app",
            "close_open",
            "pin_open",
            "app_menu",
            "run_app_entry",
            "reveal_app",
            "list_icon_packs",
            "reload_icon_packs",
            "open_icon_packs_folder",
            "launcher_status",
            "clear_launcher_history",
            "set_launcher_hotkey",
            "capture_launcher_hotkey",
            // The E2E suite's, registered only in the test build (`e2e.rs`).
            "e2e_dock_view",
            "e2e_press_dock",
            "e2e_launcher_view",
            "e2e_launcher_toggle",
            "e2e_launcher_type",
            "e2e_launcher_key",
        ]),
    ))
    .expect("failed to run tauri-build");
}
