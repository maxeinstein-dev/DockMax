//! The builder of the app manages the state the windows and commands read
//! before any `setup`, plugin or app. A `state()` on a missing state aborts
//! the startup ("state() called before manage()"), which happened once with
//! the panel's state. Built on the mock runtime: no window, no WebView.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use dockyard_lib::{managed_state, with_state};
use tauri::test::{mock_builder, mock_context, noop_assets};

const ALL: [&str; 1] = ["Mutex<PanelState>"];

#[test]
fn the_state_is_managed_before_any_setup() {
    // The earliest code that can read the state is a plugin's `setup`, run
    // by `build`; the app's `setup` only runs later, in `run`, so it can
    // never be earlier than what this checks.
    let setups = Arc::new(AtomicU8::new(0));
    let in_plugin = Arc::clone(&setups);
    let app = with_state(mock_builder())
        .plugin(
            tauri::plugin::Builder::<_, ()>::new("probe")
                .setup(move |app, _api| {
                    assert_eq!(managed_state(app), ALL, "in a plugin's setup");
                    in_plugin.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                })
                .build(),
        )
        .build(mock_context(noop_assets()))
        .expect("the mock app builds");
    assert_eq!(setups.load(Ordering::SeqCst), 1, "the plugin's setup ran");
    assert_eq!(managed_state(&app), ALL);
}
