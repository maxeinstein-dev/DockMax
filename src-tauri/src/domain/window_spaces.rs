//! Which workspace each window belongs to, and what a switch hides and shows
//! (spec 0013 C2). Pure rules: the windows are read and shown or hidden in
//! `windows/window_spaces.rs`.
//!
//! A window belongs to the workspace its monitor showed when it appeared; a
//! window taken to another monitor joins the workspace that monitor shows.
//! A switch on a monitor parks (hides) the windows there of other workspaces
//! and brings back the ones parked there of the workspace now shown.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// What is known of a window that is on the screen: its workspace and the
/// monitor (its device name) it was on at the last look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tracked {
    pub workspace: String,
    pub monitor: String,
}

/// A window hidden by a switch: to be shown again when its monitor shows its
/// workspace, or when the Dockyard lets go of everything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parked {
    pub hwnd: isize,
    pub workspace: String,
    pub monitor: String,
    /// It was minimized: it comes back minimized.
    pub minimized: bool,
}

/// Brings `known` up to date with the windows on the screen now (`now`: each
/// one and its monitor). A window seen for the first time joins the workspace
/// its monitor shows (`shown`); one that moved to another monitor joins that
/// monitor's; one no longer on the screen is forgotten, unless it is parked.
/// A monitor that shows no workspace (they can't be read) changes nothing.
pub fn follow(
    known: &mut HashMap<isize, Tracked>,
    now: &[(isize, String)],
    parked: &[Parked],
    shown: &dyn Fn(&str) -> Option<String>,
) {
    known.retain(|hwnd, _| {
        now.iter().any(|(seen, _)| seen == hwnd) || parked.iter().any(|p| p.hwnd == *hwnd)
    });
    for (hwnd, monitor) in now {
        let moved = known
            .get(hwnd)
            .is_none_or(|tracked| &tracked.monitor != monitor);
        if !moved {
            continue;
        }
        if let Some(workspace) = shown(monitor) {
            known.insert(
                *hwnd,
                Tracked {
                    workspace,
                    monitor: monitor.clone(),
                },
            );
        }
    }
}

/// The windows on the monitor `monitor` (of `now`, the ones on the screen)
/// that a switch to `target` parks: those of another workspace. A window not
/// known yet (it appeared a moment ago) belongs to what the monitor showed:
/// another workspace too.
pub fn to_park(
    known: &HashMap<isize, Tracked>,
    now: &[(isize, String)],
    monitor: &str,
    target: &str,
) -> Vec<isize> {
    now.iter()
        .filter(|(_, on)| on == monitor)
        .filter(|(hwnd, _)| {
            known
                .get(hwnd)
                .is_none_or(|tracked| tracked.workspace != target)
        })
        .map(|(hwnd, _)| *hwnd)
        .collect()
}

/// The windows parked on the monitor `monitor` that a switch to `target`
/// brings back.
pub fn to_unpark<'a>(parked: &'a [Parked], monitor: &str, target: &str) -> Vec<&'a Parked> {
    parked
        .iter()
        .filter(|p| p.monitor == monitor && p.workspace == target)
        .collect()
}

/// Whether the window `hwnd` counts on the dock of the workspace
/// `workspace`: it belongs to it, or it is not known yet (it appeared a
/// moment ago, before a look gave it a workspace). Every window counts when
/// the dock shows no workspace.
pub fn counts_for(known: &HashMap<isize, Tracked>, hwnd: isize, workspace: Option<&str>) -> bool {
    match (workspace, known.get(&hwnd)) {
        (None, _) | (_, None) => true,
        (Some(workspace), Some(tracked)) => tracked.workspace == workspace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: &str = r"\\.\DISPLAY1";
    const RIGHT: &str = r"\\.\DISPLAY2";

    fn shown(monitor: &str) -> Option<String> {
        Some(if monitor == LEFT { "work" } else { "home" }.to_owned())
    }

    fn on(hwnd: isize, monitor: &str) -> (isize, String) {
        (hwnd, monitor.to_owned())
    }

    #[test]
    fn a_new_window_joins_what_its_monitor_shows() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT), on(2, RIGHT)], &[], &shown);
        assert_eq!(known[&1].workspace, "work");
        assert_eq!(known[&2].workspace, "home");
    }

    #[test]
    fn a_window_keeps_its_workspace_while_on_its_monitor() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT)], &[], &shown);
        // The monitor shows another workspace now: the window stays where
        // it was.
        follow(&mut known, &[on(1, LEFT)], &[], &|_| Some("other".into()));
        assert_eq!(known[&1].workspace, "work");
    }

    #[test]
    fn a_window_taken_to_another_monitor_joins_its_workspace() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT)], &[], &shown);
        follow(&mut known, &[on(1, RIGHT)], &[], &shown);
        assert_eq!(known[&1].workspace, "home");
        assert_eq!(known[&1].monitor, RIGHT);
    }

    #[test]
    fn a_closed_window_is_forgotten_but_a_parked_one_is_kept() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT), on(2, LEFT)], &[], &shown);
        let parked = [Parked {
            hwnd: 2,
            workspace: "work".into(),
            monitor: LEFT.into(),
            minimized: false,
        }];
        follow(&mut known, &[], &parked, &shown);
        assert!(!known.contains_key(&1));
        assert!(known.contains_key(&2));
    }

    #[test]
    fn without_workspaces_nothing_is_tracked() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT)], &[], &|_| None);
        assert!(known.is_empty());
    }

    #[test]
    fn a_switch_parks_the_other_workspaces_windows_on_its_monitor_only() {
        let mut known = HashMap::new();
        let now = [on(1, LEFT), on(2, LEFT), on(3, RIGHT), on(4, LEFT)];
        follow(&mut known, &now[..3], &[], &shown);
        known.get_mut(&2).unwrap().workspace = "focus".into();
        // 1 is "work", 2 is "focus", 3 is on the other monitor, 4 is new.
        assert_eq!(to_park(&known, &now, LEFT, "focus"), [1, 4]);
        assert_eq!(to_park(&known, &now, LEFT, "work"), [2, 4]);
    }

    #[test]
    fn a_switch_brings_back_what_it_parked_there() {
        let parked = vec![
            Parked {
                hwnd: 1,
                workspace: "work".into(),
                monitor: LEFT.into(),
                minimized: false,
            },
            Parked {
                hwnd: 2,
                workspace: "work".into(),
                monitor: RIGHT.into(),
                minimized: true,
            },
            Parked {
                hwnd: 3,
                workspace: "home".into(),
                monitor: LEFT.into(),
                minimized: false,
            },
        ];
        let back: Vec<isize> = to_unpark(&parked, LEFT, "work")
            .iter()
            .map(|p| p.hwnd)
            .collect();
        assert_eq!(back, [1]);
    }

    #[test]
    fn a_dock_counts_its_workspaces_windows() {
        let mut known = HashMap::new();
        follow(&mut known, &[on(1, LEFT), on(2, RIGHT)], &[], &shown);
        assert!(counts_for(&known, 1, Some("work")));
        assert!(!counts_for(&known, 2, Some("work")));
        // Not known yet, or no workspaces: it counts.
        assert!(counts_for(&known, 9, Some("work")));
        assert!(counts_for(&known, 2, None));
    }
}
