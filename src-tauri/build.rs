fn main() {
    // Declaring the app commands makes each one require an explicit
    // permission (`allow-<command>`) in a capability.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "toggle_panel",
            "hide_panel",
            "panel_ready",
            "set_dock_layout",
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
        ]),
    ))
    .expect("failed to run tauri-build");
}
