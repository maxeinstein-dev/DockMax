fn main() {
    // Declaring the app commands makes each one require an explicit
    // permission (`allow-<command>`) in a capability.
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "toggle_panel",
            "hide_panel",
            "set_dock_hit_region",
            "get_settings",
            "update_settings",
        ]),
    ))
    .expect("failed to run tauri-build");
}
