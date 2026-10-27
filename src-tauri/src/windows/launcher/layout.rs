//! The launcher's measures and colors (spec 0012 §4), in logical pixels:
//! pure, so they are tested without a window.

use crate::windows::dock::Rect;

/// A color as red, green and blue.
pub type Rgb = (u8, u8, u8);

/// The window's width.
pub const WIDTH: f64 = 640.0;
/// The search box: its height, and the text field in it.
pub const BOX: f64 = 56.0;
pub const EDIT_LEFT: f64 = 52.0;
pub const EDIT_RIGHT: f64 = 20.0;
pub const EDIT_HEIGHT: f64 = 30.0;
/// The size of the text typed, and of the search glyph before it.
pub const EDIT_FONT: f64 = 20.0;
pub const GLYPH_FONT: f64 = 18.0;
/// The list under the box: its padding, a row, the title above the rows,
/// the gap at the sides, and an icon.
pub const LIST_PAD: f64 = 8.0;
pub const ROW: f64 = 48.0;
pub const TITLE: f64 = 24.0;
pub const SIDE: f64 = 8.0;
pub const ICON: f64 = 32.0;

/// The colors of one look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub background: Rgb,
    pub text: Rgb,
    pub dim: Rgb,
    pub selection: Rgb,
    pub selection_text: Rgb,
    /// Windows' contrast mode: the selected row also gets a border.
    pub contrast: bool,
}

/// The colors when Windows shows apps dark or light: a plain plate, and the
/// selected row in the dock's blue, as the search of the WhiteSur theme.
pub fn colors(dark: bool) -> Colors {
    if dark {
        Colors {
            background: (0x24, 0x24, 0x26),
            text: (0xF5, 0xF5, 0xF7),
            dim: (0xA1, 0xA1, 0xA6),
            selection: (0x0A, 0x84, 0xFF),
            selection_text: (0xFF, 0xFF, 0xFF),
            contrast: false,
        }
    } else {
        Colors {
            background: (0xF6, 0xF6, 0xF8),
            text: (0x1C, 0x1C, 0x1E),
            dim: (0x6E, 0x6E, 0x73),
            selection: (0x00, 0x6E, 0xE6),
            selection_text: (0xFF, 0xFF, 0xFF),
            contrast: false,
        }
    }
}

/// The colors in Windows' contrast mode: the system's own.
pub fn contrast_colors(
    window: Rgb,
    text: Rgb,
    gray: Rgb,
    highlight: Rgb,
    highlight_text: Rgb,
) -> Colors {
    Colors {
        background: window,
        text,
        dim: gray,
        selection: highlight,
        selection_text: highlight_text,
        contrast: true,
    }
}

/// The height of the list with `rows` rows, or one line of text when there
/// are none, and the title above them when `title`.
pub fn list_height(rows: usize, title: bool) -> f64 {
    let title = if title { TITLE } else { 0.0 };
    2.0 * LIST_PAD + title + rows.max(1) as f64 * ROW
}

/// The row at `y` (logical pixels from the list's top), if any.
pub fn row_at(y: f64, rows: usize, title: bool) -> Option<usize> {
    let top = LIST_PAD + if title { TITLE } else { 0.0 };
    if y < top {
        return None;
    }
    let index = ((y - top) / ROW).floor() as usize;
    (index < rows).then_some(index)
}

/// Where the window goes on `monitor` (physical pixels, at `scale`) for a
/// height of `height` logical pixels: centered across, its top a quarter of
/// the way down, as the Ulauncher. The window's top stays put as the list
/// grows.
pub fn place(monitor: Rect, scale: f64, height: f64) -> Rect {
    let width = (WIDTH * scale).round() as u32;
    let height = (height * scale).round() as u32;
    let width = width.min(monitor.width);
    let x = monitor.x + (monitor.width.saturating_sub(width) / 2) as i32;
    let y = monitor.y + (monitor.height / 4) as i32;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// The shortcut that opens the row `index` (0 based): Alt and its number,
/// for the first nine.
pub fn shortcut(index: usize) -> Option<String> {
    (index < 9).then(|| format!("Alt+{}", index + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Rect = Rect {
        x: 1920,
        y: 0,
        width: 2560,
        height: 1440,
    };

    #[test]
    fn the_window_is_centered_a_quarter_down() {
        let placed = place(MONITOR, 1.25, 200.0);
        assert_eq!(placed.width, 800);
        assert_eq!(placed.height, 250);
        assert_eq!(placed.x, 1920 + 880);
        assert_eq!(placed.y, 360);
    }

    #[test]
    fn a_narrow_monitor_keeps_the_window_inside() {
        let narrow = Rect {
            x: 0,
            y: 0,
            width: 600,
            height: 800,
        };
        let placed = place(narrow, 1.0, 100.0);
        assert_eq!((placed.x, placed.width), (0, 600));
    }

    #[test]
    fn the_list_is_one_line_tall_without_rows() {
        assert_eq!(list_height(0, false), 2.0 * LIST_PAD + ROW);
        assert_eq!(list_height(8, true), 2.0 * LIST_PAD + TITLE + 8.0 * ROW);
    }

    #[test]
    fn the_row_under_the_pointer_skips_the_title() {
        assert_eq!(row_at(LIST_PAD - 1.0, 3, false), None);
        assert_eq!(row_at(LIST_PAD, 3, false), Some(0));
        assert_eq!(row_at(LIST_PAD + ROW, 3, false), Some(1));
        assert_eq!(row_at(LIST_PAD + 3.0 * ROW, 3, false), None);
        assert_eq!(row_at(LIST_PAD + 10.0, 3, true), None);
        assert_eq!(row_at(LIST_PAD + TITLE, 3, true), Some(0));
    }

    #[test]
    fn only_the_first_nine_rows_have_a_shortcut() {
        assert_eq!(shortcut(0).as_deref(), Some("Alt+1"));
        assert_eq!(shortcut(8).as_deref(), Some("Alt+9"));
        assert_eq!(shortcut(9), None);
    }
}
