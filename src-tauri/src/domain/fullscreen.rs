//! Whether a monitor is taken by a program in full screen (spec 0010 D4a): the
//! dock on that monitor steps aside. Pure rules: the windows are read in
//! `windows/fullscreen.rs`.
//!
//! The rule looks at each monitor on its own, at the windows on it, and not at
//! the window in front: using the other monitor (and so moving the front
//! there) must not bring the dock back over a video in full screen.

/// A rectangle in physical pixels: left, top, right, bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// A window that would have a taskbar button, as the full-screen rule sees it:
/// the monitor Windows counts it on and its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub monitor: isize,
    pub bounds: Bounds,
}

/// Whether `window` covers all of `monitor`.
pub fn covers(window: Bounds, monitor: Bounds) -> bool {
    window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

/// Whether `monitor` (its handle and its rectangle) is in full screen: the
/// topmost of the `candidates` (in stacking order, the top first) that is on
/// it covers all of it. A maximized window leaves the dock's band free, so it
/// is not full screen; a small window above the video (a picture in picture
/// counts only if it has a taskbar button) makes the monitor not full screen
/// any more, as the user is working above it.
pub fn is_fullscreen(candidates: &[Candidate], monitor: isize, bounds: Bounds) -> bool {
    candidates
        .iter()
        .find(|candidate| candidate.monitor == monitor)
        .is_some_and(|top| covers(top.bounds, bounds))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIMARY: Bounds = Bounds {
        left: 0,
        top: 0,
        right: 2560,
        bottom: 1080,
    };
    const SECOND: Bounds = Bounds {
        left: -1920,
        top: 6,
        right: 0,
        bottom: 1206,
    };

    fn on(monitor: isize, left: i32, top: i32, right: i32, bottom: i32) -> Candidate {
        Candidate {
            monitor,
            bounds: Bounds {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    #[test]
    fn a_window_that_covers_the_monitor_takes_it() {
        assert!(covers(PRIMARY, PRIMARY));
        assert!(covers(
            Bounds {
                left: -8,
                top: -8,
                right: 2568,
                bottom: 1088
            },
            PRIMARY
        ));
        assert!(!covers(
            Bounds {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1009
            },
            PRIMARY
        ));
    }

    #[test]
    fn a_video_in_full_screen_takes_its_monitor_only() {
        let windows = [on(1, 0, 0, 2560, 1080), on(2, -1800, 100, -400, 900)];

        assert!(is_fullscreen(&windows, 1, PRIMARY));
        assert!(!is_fullscreen(&windows, 2, SECOND));
    }

    #[test]
    fn using_the_other_monitor_does_not_free_the_full_screen_one() {
        // The front moved to the other monitor: its window is now the first
        // in the stacking order. The video is still the top of its monitor.
        let windows = [on(2, -1800, 100, -400, 900), on(1, 0, 0, 2560, 1080)];

        assert!(is_fullscreen(&windows, 1, PRIMARY));
    }

    #[test]
    fn a_window_above_the_video_on_its_monitor_frees_it() {
        let windows = [on(1, 200, 200, 900, 700), on(1, 0, 0, 2560, 1080)];

        assert!(!is_fullscreen(&windows, 1, PRIMARY));
    }

    #[test]
    fn a_maximized_window_leaves_the_dock_its_band() {
        // Maximized: the work area plus the frame, short of the dock's band.
        let windows = [on(1, -8, -8, 2568, 1017)];

        assert!(!is_fullscreen(&windows, 1, PRIMARY));
    }

    #[test]
    fn a_monitor_with_no_window_is_not_in_full_screen() {
        assert!(!is_fullscreen(&[], 1, PRIMARY));
        assert!(!is_fullscreen(&[on(2, -1920, 6, 0, 1206)], 1, PRIMARY));
    }
}
