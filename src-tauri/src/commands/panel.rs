use std::sync::Mutex;
use std::time::Instant;

use tauri::{AppHandle, Manager, WebviewWindow};

use crate::commands::widgets;
use crate::domain::widgets::WidgetError;
use crate::persistence::database::Storage;
use crate::windows::dock;
use crate::windows::panel::{
    self, Action, LogicalRect, Panel, PanelGeometry, PanelId, PanelInset, PanelSize,
};

/// The panel `id` names, checked against the storage: a widget panel opens
/// only for an instance the dock has, of a kind usable in this phase
/// (`notFound` or `notCreatable` otherwise). Settings and About always
/// open.
fn resolve(storage: &Storage, id: PanelId) -> Result<Panel, WidgetError> {
    match id {
        PanelId::Settings => Ok(Panel::Settings),
        PanelId::About => Ok(Panel::About),
        PanelId::Tray => Ok(Panel::Tray),
        PanelId::Widget(id) => {
            let kind = widgets::panel_kind(storage, &id)?;
            Ok(Panel::Widget { id, kind })
        }
        PanelId::App(id) => {
            // Only a program the dock has pinned has a menu.
            let snapshot = storage.apps_snapshot().map_err(|_| WidgetError::NotFound)?;
            if snapshot.apps.iter().any(|app| app.id == id) {
                Ok(Panel::App { id })
            } else {
                Err(WidgetError::NotFound)
            }
        }
        PanelId::Open(key) => {
            // Only a program that is open now, and that is not pinned, has
            // this menu.
            if is_open_key(&key) {
                Ok(Panel::Open { key })
            } else {
                Err(WidgetError::NotFound)
            }
        }
    }
}

/// Whether `key` names an open program that is not pinned.
fn is_open_key(key: &str) -> bool {
    #[cfg(windows)]
    {
        crate::windows::open_apps::app_id_of_other(key).is_some()
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        false
    }
}

/// The dock's item was clicked (spec 0011 H3: the dock is native, so this is
/// a call, not a command). `anchor` is the item's rectangle relative to the
/// dock's window, in logical pixels. Opening asks the panel page to render
/// first (`panel-show`); the window shows on `panel_ready` or after
/// `panel::READY_TIMEOUT`.
///
/// A widget instance the storage doesn't have (removed, or the widgets
/// unavailable in this run) is refused with an error, and nothing changes:
/// the open panel, if any, stays open.
pub fn toggle(app: &AppHandle, panel_id: PanelId, anchor: LogicalRect) -> Result<(), String> {
    let app = app.clone();
    // Configurações is an app of its own now (spec 0011 H1): the dock's item
    // opens it, and the open panel, if any, closes.
    if panel_id == PanelId::Settings {
        let action = panel::lock(&app).hide();
        if let Err(error) = panel::apply(&app, action) {
            log::warn!("panel: failed to close before the settings: {error}");
        }
        return crate::windows::settings_window::open(&app).map_err(|error| {
            log::warn!("settings: the window could not be opened: {error}");
            "the settings could not be opened".to_string()
        });
    }
    // The storage lock is released before the panel's.
    let storage = app.state::<Mutex<Storage>>();
    let resolved = resolve(&widgets::lock(&storage), panel_id);
    let panel = resolved.map_err(|error| {
        log::warn!("panel: refused to open a widget panel: {error}");
        "no such widget".to_string()
    })?;
    let action = panel::lock(&app).toggle(panel, anchor, Instant::now());
    let show = matches!(action, Action::Prepare(..));
    if show && !dock::is_visible() {
        // A click queued before the dock was hidden from the tray: a panel
        // anchored to a hidden dock stays closed.
        panel::close(&app);
        return Ok(());
    }
    if let Err(error) = panel::apply(&app, action) {
        log::warn!("panel: failed to toggle: {error}");
        if show {
            panel::close(&app);
        }
        return Err("could not open the panel".into());
    }
    Ok(())
}

/// Called by the panel to close itself (Esc).
#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<(), String> {
    let action = panel::lock(&app).hide();
    panel::apply(&app, action).map_err(|error| {
        log::warn!("panel: failed to hide: {error}");
        "could not close the panel".to_string()
    })
}

/// Called by the panel once the content `panel-show` asked for has
/// rendered, with that show's `wait`, the window size it needs and the
/// transparent inset around its visible card, in logical pixels. Shows the
/// panel there; a signal for a panel that is no longer open, or for an
/// earlier show of it, is ignored. Called again for the open panel, places
/// it again.
#[tauri::command]
pub fn panel_ready(
    app: AppHandle,
    window: WebviewWindow,
    panel_id: PanelId,
    wait: u64,
    size: PanelSize,
    inset: PanelInset,
) -> Result<(), String> {
    if window.label() != panel::LABEL {
        return Err("only the panel reports that it rendered".into());
    }
    // `size` and `inset` were checked when deserialized.
    let action = panel::lock(&app).ready(panel_id, wait, PanelGeometry::new(size, inset));
    if action == Action::Nothing {
        log::debug!("panel: ignored a stale ready signal");
        return Ok(());
    }
    panel::reveal_or_close(&app, action).map_err(|error| {
        log::warn!("panel: failed to show: {error}");
        "could not open the panel".to_string()
    })
}

/// Called by the panel's page once it listens for `panel-show` (spec 0011
/// H2): the show that waited for the window to be made goes out now.
#[tauri::command]
pub fn panel_mounted(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    if window.label() != panel::LABEL {
        return Err("only the panel mounts".into());
    }
    panel::page_mounted(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::domain::widgets::{UnavailableReason, WidgetId, WidgetKind};

    #[test]
    fn settings_and_about_always_open() {
        let storage = Storage::unavailable(UnavailableReason::OpenFailed);
        assert_eq!(resolve(&storage, PanelId::Settings), Ok(Panel::Settings));
        assert_eq!(resolve(&storage, PanelId::About), Ok(Panel::About));
    }

    #[test]
    fn a_widget_panel_opens_with_the_kind_of_its_instance() {
        let dir = tempdir().unwrap();
        let storage = widgets::open_storage::<String>(Ok(dir.path().to_path_buf()));
        let seeded = widgets::snapshot(&storage).unwrap().widgets;
        assert!(!seeded.is_empty());

        for instance in seeded {
            let id = instance.id.clone();
            assert_eq!(
                resolve(&storage, PanelId::Widget(id.clone())),
                Ok(Panel::Widget {
                    id,
                    kind: instance.config.kind()
                })
            );
        }
    }

    #[test]
    fn a_widget_panel_for_a_missing_instance_is_refused() {
        let dir = tempdir().unwrap();
        let mut storage = widgets::open_storage::<String>(Ok(dir.path().to_path_buf()));
        let note = widgets::snapshot(&storage)
            .unwrap()
            .widgets
            .into_iter()
            .find(|instance| instance.config.kind() == WidgetKind::Note)
            .unwrap();
        widgets::remove(&mut storage, note.id.as_str(), None).unwrap();

        assert_eq!(
            resolve(&storage, PanelId::Widget(note.id)),
            Err(WidgetError::NotFound)
        );
        assert_eq!(
            resolve(&storage, PanelId::Widget(WidgetId::new())),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn an_app_panel_opens_only_for_a_pinned_program() {
        let dir = tempdir().unwrap();
        let mut storage = widgets::open_storage::<String>(Ok(dir.path().to_path_buf()));
        let pinned =
            crate::commands::apps::pin(&mut storage, "Some.App!App", "Some App", None).unwrap();

        assert_eq!(
            resolve(&storage, PanelId::App(pinned.id.clone())),
            Ok(Panel::App {
                id: pinned.id.clone()
            })
        );
        // A widget's id is not a pinned program's, and the other way round.
        let widget = widgets::snapshot(&storage).unwrap().widgets.remove(0);
        assert_eq!(
            resolve(&storage, PanelId::App(widget.id.clone())),
            Err(WidgetError::NotFound)
        );
        assert_eq!(
            resolve(&storage, PanelId::Widget(pinned.id.clone())),
            Err(WidgetError::NotFound)
        );
        crate::commands::apps::unpin(&mut storage, pinned.id.as_str(), None).unwrap();
        assert_eq!(
            resolve(&storage, PanelId::App(pinned.id)),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn no_menu_opens_for_a_program_that_is_not_open() {
        let storage = Storage::unavailable(UnavailableReason::OpenFailed);

        // Well formed but nothing is open under it, and not a key at all.
        for key in ["00000000000000aa", "", "not-a-key", "C:\\x.exe"] {
            assert_eq!(
                resolve(&storage, PanelId::Open(key.into())),
                Err(WidgetError::NotFound),
                "{key:?}"
            );
        }
    }

    #[test]
    fn without_the_storage_no_app_panel_opens() {
        let storage = Storage::unavailable(UnavailableReason::OpenFailed);

        assert_eq!(
            resolve(&storage, PanelId::App(WidgetId::new())),
            Err(WidgetError::NotFound)
        );
    }

    #[test]
    fn without_the_storage_no_widget_panel_opens() {
        let storage = Storage::unavailable(UnavailableReason::NewerVersion);
        assert_eq!(
            resolve(&storage, PanelId::Widget(WidgetId::new())),
            Err(WidgetError::NotFound)
        );
    }
}
