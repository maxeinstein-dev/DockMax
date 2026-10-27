//! Workspaces (spec 0013): each one is a dock of its own, with its name, its
//! color and the items it shows in order. Pure rules: the rows are kept in
//! `persistence/workspaces_repo.rs`.
//!
//! An item is named by the key the dock already uses (`app:<id>` for a
//! pinned program, `widget:<id>` for a widget): the programs and the widgets
//! stay one list each, and a workspace says which of them it shows. The same
//! one may be in several workspaces; it is removed for good only when no
//! workspace has it any more.

use serde::Serialize;

/// The most workspaces: `Ctrl+Alt+1…9` reaches each one.
pub const MAX_WORKSPACES: usize = 9;

/// The longest name, in characters.
pub const MAX_NAME: usize = 32;

/// The most items a workspace shows: the programs and the widgets the dock
/// can have.
pub const MAX_ITEMS: usize = 200;

/// The longest item key: a kind and an id of 36 characters.
pub const MAX_KEY: usize = 64;

/// The colors a workspace can have, in the order new ones take them: the
/// PrimoDock's three (blue, orange, green), then purple, pink, red, yellow
/// and gray.
pub const PALETTE: [&str; 8] = [
    "#0A84FF", "#FF9F0A", "#30D158", "#BF5AF2", "#FF375F", "#FF453A", "#FFD60A", "#8E8E93",
];

/// The name of the workspace every dock starts with.
pub const FIRST_NAME: &str = "Principal";

/// One workspace, as the interface sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub color: String,
    /// The keys of its items, in dock order.
    pub items: Vec<String>,
}

/// How a change to the workspaces fails: `{ "code": "lastWorkspace" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace not found")]
    NotFound,
    #[error("workspace limit reached")]
    LimitReached,
    #[error("invalid workspace name")]
    InvalidName,
    #[error("invalid workspace color")]
    InvalidColor,
    #[error("the last workspace can't be deleted")]
    LastWorkspace,
    #[error("workspace storage unavailable")]
    Unavailable,
    #[error("workspace storage could not be written")]
    WriteFailed,
}

/// What the interface gets of a [`WorkspaceError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceErrorCode {
    NotFound,
    LimitReached,
    InvalidName,
    InvalidColor,
    LastWorkspace,
    Unavailable,
    WriteFailed,
}

impl WorkspaceError {
    pub fn code(self) -> WorkspaceErrorCode {
        match self {
            WorkspaceError::NotFound => WorkspaceErrorCode::NotFound,
            WorkspaceError::LimitReached => WorkspaceErrorCode::LimitReached,
            WorkspaceError::InvalidName => WorkspaceErrorCode::InvalidName,
            WorkspaceError::InvalidColor => WorkspaceErrorCode::InvalidColor,
            WorkspaceError::LastWorkspace => WorkspaceErrorCode::LastWorkspace,
            WorkspaceError::Unavailable => WorkspaceErrorCode::Unavailable,
            WorkspaceError::WriteFailed => WorkspaceErrorCode::WriteFailed,
        }
    }
}

impl From<crate::domain::widgets::WidgetError> for WorkspaceError {
    fn from(error: crate::domain::widgets::WidgetError) -> Self {
        match error {
            crate::domain::widgets::WidgetError::Unavailable => WorkspaceError::Unavailable,
            _ => WorkspaceError::WriteFailed,
        }
    }
}

/// A name the user may give: one line, no control characters, no spaces at
/// the ends, 1 to `MAX_NAME` characters.
pub fn is_valid_name(name: &str) -> bool {
    let length = name.chars().count();
    (1..=MAX_NAME).contains(&length) && name.trim() == name && !name.chars().any(char::is_control)
}

/// A color of the palette (case does not matter).
pub fn is_palette_color(color: &str) -> bool {
    PALETTE
        .iter()
        .any(|known| known.eq_ignore_ascii_case(color))
}

/// An item key a workspace can hold: a pinned program or a widget.
pub fn is_item_key(key: &str) -> bool {
    key.len() <= MAX_KEY && (key.starts_with("app:") || key.starts_with("widget:"))
}

/// The color a new workspace takes: the first of the palette no workspace
/// has, or, with all taken, the one after the last workspace's.
pub fn next_color(taken: &[&str]) -> &'static str {
    if let Some(free) = PALETTE
        .iter()
        .find(|color| !taken.iter().any(|used| used.eq_ignore_ascii_case(color)))
    {
        return free;
    }
    let last = taken
        .last()
        .and_then(|last| PALETTE.iter().position(|c| c.eq_ignore_ascii_case(last)))
        .unwrap_or(0);
    PALETTE[(last + 1) % PALETTE.len()]
}

/// The name a new workspace takes: "Espaço 2", "Espaço 3"…, the first not
/// taken.
pub fn next_name(taken: &[&str]) -> String {
    (2..)
        .map(|number| format!("Espaço {number}"))
        .find(|name| !taken.contains(&name.as_str()))
        .unwrap_or_default()
}

/// The order the first workspace starts with: what the dock showed before
/// there were workspaces. `order` (the old `dock_order`) says, by its keys of
/// each kind, where the next program and where the next widget go; each kind
/// comes in its own order, and those `order` doesn't name go at the end, the
/// programs first.
pub fn initial_order(order: &[String], apps: &[String], widgets: &[String]) -> Vec<String> {
    let mut apps_left = apps.iter();
    let mut widgets_left = widgets.iter();
    let mut mixed = Vec::new();
    for key in order {
        let next = if apps.contains(key) {
            apps_left.next()
        } else if widgets.contains(key) {
            widgets_left.next()
        } else {
            None
        };
        mixed.extend(next.cloned());
    }
    mixed.extend(apps_left.cloned());
    mixed.extend(widgets_left.cloned());
    mixed
}

/// `items` with `key` one place up or down among the keys of its own kind
/// (the lists of Configurações show each kind apart); `None` when it is not
/// there or already at that end.
pub fn shifted(items: &[String], key: &str, up: bool) -> Option<Vec<String>> {
    let kind = |key: &str| key.split(':').next().unwrap_or_default().to_owned();
    let own = kind(key);
    let from = items.iter().position(|item| item == key)?;
    let same: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| kind(item) == own)
        .map(|(index, _)| index)
        .collect();
    let at = same.iter().position(|&index| index == from)?;
    let other = if up {
        same.get(at.checked_sub(1)?)?
    } else {
        same.get(at + 1)?
    };
    let mut moved = items.to_vec();
    moved.swap(from, *other);
    Some(moved)
}

/// The workspace after (`forward`) or before the one at `current`, among
/// `count`, without going round: `None` at either end.
pub fn neighbor(count: usize, current: usize, forward: bool) -> Option<usize> {
    if forward {
        (current + 1 < count).then_some(current + 1)
    } else {
        current.checked_sub(1)
    }
}

impl From<WorkspaceError> for crate::domain::apps::AppError {
    fn from(error: WorkspaceError) -> Self {
        use crate::domain::apps::AppError;
        match error {
            WorkspaceError::NotFound => AppError::NotFound,
            WorkspaceError::LimitReached => AppError::LimitReached,
            WorkspaceError::Unavailable => AppError::Unavailable,
            _ => AppError::WriteFailed,
        }
    }
}

impl From<WorkspaceError> for crate::domain::widgets::WidgetError {
    fn from(error: WorkspaceError) -> Self {
        use crate::domain::widgets::WidgetError;
        match error {
            WorkspaceError::NotFound => WidgetError::NotFound,
            WorkspaceError::LimitReached => WidgetError::LimitReached,
            WorkspaceError::Unavailable => WidgetError::Unavailable,
            _ => WidgetError::WriteFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    #[test]
    fn names_are_one_short_line() {
        assert!(is_valid_name("Trabalho"));
        assert!(is_valid_name("Foco 2"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name(" Foco"));
        assert!(!is_valid_name("Foco\n"));
        assert!(!is_valid_name(&"a".repeat(MAX_NAME + 1)));
        assert!(is_valid_name(&"é".repeat(MAX_NAME)));
    }

    #[test]
    fn colors_come_from_the_palette() {
        assert!(is_palette_color("#0A84FF"));
        assert!(is_palette_color("#0a84ff"));
        assert!(!is_palette_color("#123456"));
        assert_eq!(next_color(&[]), "#0A84FF");
        assert_eq!(next_color(&["#0A84FF"]), "#FF9F0A");
        assert_eq!(next_color(&["#FF9F0A"]), "#0A84FF");
        // All taken: the one after the last.
        assert_eq!(next_color(&PALETTE), "#0A84FF");
        assert_eq!(
            next_color(&[
                PALETTE[2], PALETTE[0], PALETTE[1], PALETTE[3], PALETTE[4], PALETTE[5], PALETTE[6],
                PALETTE[7], PALETTE[2]
            ]),
            PALETTE[3]
        );
    }

    #[test]
    fn new_workspaces_get_the_next_free_name() {
        assert_eq!(next_name(&["Principal"]), "Espaço 2");
        assert_eq!(next_name(&["Principal", "Espaço 2"]), "Espaço 3");
        assert_eq!(next_name(&["Espaço 3"]), "Espaço 2");
    }

    #[test]
    fn item_keys_are_programs_and_widgets() {
        assert!(is_item_key("app:0192"));
        assert!(is_item_key("widget:0192"));
        assert!(!is_item_key("open:chrome"));
        assert!(!is_item_key("settings"));
        assert!(!is_item_key(&format!("app:{}", "x".repeat(MAX_KEY))));
    }

    #[test]
    fn the_first_workspace_keeps_what_the_dock_showed() {
        let apps = keys(&["app:a", "app:b"]);
        let widgets = keys(&["widget:1", "widget:2"]);
        // Nothing dragged yet: the programs, then the widgets.
        assert_eq!(
            initial_order(&[], &apps, &widgets),
            keys(&["app:a", "app:b", "widget:1", "widget:2"])
        );
        // A widget dragged before the programs.
        let order = keys(&["widget:1", "app:a", "app:b", "widget:2"]);
        assert_eq!(initial_order(&order, &apps, &widgets), order);
        // Keys of items gone are skipped; new items go at the end.
        let order = keys(&["widget:9", "app:a"]);
        assert_eq!(
            initial_order(&order, &apps, &widgets),
            keys(&["app:a", "app:b", "widget:1", "widget:2"])
        );
    }

    #[test]
    fn an_item_moves_among_its_own_kind() {
        let items = keys(&["app:a", "widget:1", "app:b", "widget:2"]);
        assert_eq!(
            shifted(&items, "app:b", true),
            Some(keys(&["app:b", "widget:1", "app:a", "widget:2"]))
        );
        assert_eq!(
            shifted(&items, "widget:1", false),
            Some(keys(&["app:a", "widget:2", "app:b", "widget:1"]))
        );
        assert_eq!(shifted(&items, "app:a", true), None);
        assert_eq!(shifted(&items, "widget:2", false), None);
        assert_eq!(shifted(&items, "app:z", true), None);
    }

    #[test]
    fn switching_does_not_go_round() {
        assert_eq!(neighbor(3, 0, true), Some(1));
        assert_eq!(neighbor(3, 2, true), None);
        assert_eq!(neighbor(3, 0, false), None);
        assert_eq!(neighbor(3, 2, false), Some(1));
    }
}
