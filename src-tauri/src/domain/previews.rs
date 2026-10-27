//! Where the window previews go (spec 0015): the card over a program's icon,
//! and a cell in it for each window, with its title, its close button and
//! its live thumbnail. Pure: `windows/previews.rs` shows the card and asks
//! the DWM for the thumbnails.
//!
//! Every length is in physical pixels of the screen; the sizes below are
//! logical and grow with the monitor's scale.

use crate::domain::dock_layout::Area;
use crate::domain::settings::DockEdge;

/// The largest thumbnail, in logical pixels.
pub const THUMB_WIDTH: f64 = 200.0;
pub const THUMB_HEIGHT: f64 = 120.0;
/// A cell is never narrower: past that, windows are left out ("+N").
pub const MIN_THUMB_WIDTH: f64 = 96.0;
/// The title's row, above the thumbnail.
pub const TITLE_HEIGHT: f64 = 28.0;
/// The close button's side, at the end of the title's row.
pub const CLOSE_SIDE: f64 = 22.0;
/// The column of "+N" at the card's end, when windows are left out.
pub const MORE_WIDTH: f64 = 44.0;
/// Inside the card, around the cells, and between two cells.
pub const PAD: f64 = 8.0;
pub const GAP: f64 = 6.0;
/// Between the icon and the card, and between the card and the screen's
/// edge.
pub const OFFSET: f64 = 10.0;
pub const MARGIN: f64 = 8.0;

/// One window's place in the card, in the card's own pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The window's index in the list given.
    pub index: usize,
    /// The whole cell: the click there brings the window to the front.
    pub area: Area,
    pub title: Area,
    pub close: Area,
    /// The thumbnail, the window's shape kept, centred in its box.
    pub thumb: Area,
}

/// The card on the screen and its cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// On the screen.
    pub area: Area,
    pub cells: Vec<Cell>,
    /// The windows left out for want of room, and where "+N" says so: a
    /// column of its own at the card's end, since a thumbnail is drawn over
    /// anything under it.
    pub more: usize,
    pub more_area: Option<Area>,
}

/// The card for windows of the sizes `sizes` (their width and height; 0 for
/// one the DWM can't measure), over the icon `icon` of a dock on `edge`, on
/// the monitor `screen`, at `scale`. `None` without windows.
pub fn card(
    icon: Area,
    edge: DockEdge,
    screen: Area,
    scale: f64,
    sizes: &[(f64, f64)],
) -> Option<Card> {
    if sizes.is_empty() {
        return None;
    }
    let pad = PAD * scale;
    let gap = GAP * scale;
    let room = (screen.width - 2.0 * MARGIN * scale).max(0.0);
    // The widest cell that lets every window in, between the largest and
    // the smallest.
    let count = sizes.len() as f64;
    let fitting = (room - 2.0 * pad - (count - 1.0) * gap) / count;
    let width = fitting.clamp(MIN_THUMB_WIDTH * scale, THUMB_WIDTH * scale);
    // How many cells of that width fit; with some left out, "+N" takes a
    // column.
    let fits = |room: f64| ((room - 2.0 * pad + gap) / (width + gap)).floor().max(0.0) as usize;
    let mut shown = fits(room).clamp(1, sizes.len());
    if shown < sizes.len() {
        shown = fits(room - (MORE_WIDTH + GAP) * scale).clamp(1, sizes.len());
    }
    let more = sizes.len() - shown;
    let more_width = if more > 0 {
        (MORE_WIDTH + GAP) * scale
    } else {
        0.0
    };
    let height = THUMB_HEIGHT * scale * (width / (THUMB_WIDTH * scale));
    let title = TITLE_HEIGHT * scale;
    let close = CLOSE_SIDE * scale;

    let cells_width = shown as f64 * width + (shown as f64 - 1.0) * gap;
    let card_width = 2.0 * pad + cells_width + more_width;
    let card_height = 2.0 * pad + title + height;
    let cells = sizes
        .iter()
        .take(shown)
        .enumerate()
        .map(|(index, &(source_width, source_height))| {
            let x = pad + index as f64 * (width + gap);
            let area = Area {
                x,
                y: pad,
                width,
                height: title + height,
            };
            Cell {
                index,
                area,
                title: Area {
                    x,
                    y: pad,
                    width: (width - close).max(0.0),
                    height: title,
                },
                close: Area {
                    x: x + width - close,
                    y: pad + (title - close) / 2.0,
                    width: close,
                    height: close,
                },
                thumb: fit(
                    Area {
                        x,
                        y: pad + title,
                        width,
                        height,
                    },
                    source_width,
                    source_height,
                ),
            }
        })
        .collect();

    // Centred on the icon, kept on the screen; over a dock at the bottom,
    // under one at the top.
    let centre = icon.x + icon.width / 2.0;
    let left_most = screen.x + MARGIN * scale;
    let right_most = screen.x + screen.width - MARGIN * scale - card_width;
    let x = (centre - card_width / 2.0).min(right_most).max(left_most);
    let y = match edge {
        DockEdge::Bottom => icon.y - OFFSET * scale - card_height,
        DockEdge::Top => icon.y + icon.height + OFFSET * scale,
    };
    let y = y.min(screen.y + screen.height - card_height).max(screen.y);
    Some(Card {
        area: Area {
            x: x.round(),
            y: y.round(),
            width: card_width.round(),
            height: card_height.round(),
        },
        cells,
        more,
        more_area: (more > 0).then_some(Area {
            x: pad + cells_width + gap,
            y: pad,
            width: MORE_WIDTH * scale,
            height: title + height,
        }),
    })
}

/// The box `room` shrunk to the shape of a `width` × `height` window,
/// centred; the whole box for one that can't be measured.
fn fit(room: Area, width: f64, height: f64) -> Area {
    if width <= 0.0 || height <= 0.0 {
        return room;
    }
    let scale = (room.width / width).min(room.height / height);
    let (fitted_width, fitted_height) = (width * scale, height * scale);
    Area {
        x: room.x + (room.width - fitted_width) / 2.0,
        y: room.y + (room.height - fitted_height) / 2.0,
        width: fitted_width,
        height: fitted_height,
    }
}

/// Which part of the card the point (in the card's pixels) is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Cell(usize),
    Close(usize),
}

/// The cell or close button at `(x, y)`; `None` between them.
pub fn hit(card: &Card, x: f64, y: f64) -> Option<Hit> {
    let inside = |area: &Area| {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    };
    card.cells.iter().find_map(|cell| {
        if inside(&cell.close) {
            Some(Hit::Close(cell.index))
        } else if inside(&cell.area) {
            Some(Hit::Cell(cell.index))
        } else {
            None
        }
    })
}

/// Whether a program with `windows` windows shows a card: none without
/// windows, and with "only with two or more" none for a single one.
pub fn shows(windows: usize, only_several: bool) -> bool {
    windows >= if only_several { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Area = Area {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };

    fn icon(x: f64, y: f64) -> Area {
        Area {
            x,
            y,
            width: 48.0,
            height: 48.0,
        }
    }

    #[test]
    fn no_window_no_card() {
        assert_eq!(
            card(icon(900.0, 1020.0), DockEdge::Bottom, SCREEN, 1.0, &[]),
            None
        );
    }

    #[test]
    fn the_card_sits_over_a_bottom_dock_centred_on_the_icon() {
        let card = card(
            icon(900.0, 1020.0),
            DockEdge::Bottom,
            SCREEN,
            1.0,
            &[(1600.0, 900.0)],
        )
        .unwrap();
        assert_eq!(card.area.width, 2.0 * PAD + THUMB_WIDTH);
        assert_eq!(card.area.height, 2.0 * PAD + TITLE_HEIGHT + THUMB_HEIGHT);
        assert_eq!(card.area.x, (924.0 - card.area.width / 2.0).round());
        assert_eq!(card.area.y + card.area.height, 1020.0 - OFFSET);
        assert_eq!(card.more, 0);
    }

    #[test]
    fn the_card_sits_under_a_top_dock() {
        let card = card(
            icon(900.0, 4.0),
            DockEdge::Top,
            SCREEN,
            1.0,
            &[(800.0, 600.0)],
        )
        .unwrap();
        assert_eq!(card.area.y, 4.0 + 48.0 + OFFSET);
    }

    #[test]
    fn the_card_stays_on_the_screen_at_its_edges() {
        let sizes = [(800.0, 600.0); 3];
        let left = card(icon(2.0, 1020.0), DockEdge::Bottom, SCREEN, 1.0, &sizes).unwrap();
        assert_eq!(left.area.x, MARGIN);
        let right = card(icon(1900.0, 1020.0), DockEdge::Bottom, SCREEN, 1.0, &sizes).unwrap();
        assert_eq!(right.area.x + right.area.width, 1920.0 - MARGIN);
        // A second monitor to the left of the first.
        let other = Area {
            x: -1280.0,
            ..SCREEN
        };
        let far = card(icon(-1278.0, 1020.0), DockEdge::Bottom, other, 1.0, &sizes).unwrap();
        assert_eq!(far.area.x, -1280.0 + MARGIN);
    }

    #[test]
    fn a_thumbnail_keeps_the_windows_shape() {
        let card = card(
            icon(900.0, 1020.0),
            DockEdge::Bottom,
            SCREEN,
            1.0,
            &[(1000.0, 1000.0), (2000.0, 500.0), (0.0, 0.0)],
        )
        .unwrap();
        let [square, wide, unknown] = [
            card.cells[0].thumb,
            card.cells[1].thumb,
            card.cells[2].thumb,
        ];
        assert_eq!((square.width, square.height), (THUMB_HEIGHT, THUMB_HEIGHT));
        assert_eq!((wide.width, wide.height), (THUMB_WIDTH, THUMB_WIDTH / 4.0));
        assert_eq!((unknown.width, unknown.height), (THUMB_WIDTH, THUMB_HEIGHT));
        // Centred in its box, under the title.
        assert_eq!(
            square.x,
            card.cells[0].area.x + (THUMB_WIDTH - THUMB_HEIGHT) / 2.0
        );
        assert_eq!(
            wide.y,
            PAD + TITLE_HEIGHT + (THUMB_HEIGHT - THUMB_WIDTH / 4.0) / 2.0
        );
    }

    #[test]
    fn many_windows_shrink_the_cells_then_leave_some_out() {
        let small = Area {
            width: 1280.0,
            ..SCREEN
        };
        let eight = card(
            icon(600.0, 1020.0),
            DockEdge::Bottom,
            small,
            1.0,
            &[(800.0, 600.0); 8],
        )
        .unwrap();
        assert_eq!(eight.cells.len(), 8);
        assert!(eight.cells[0].area.width < THUMB_WIDTH);
        assert!(eight.cells[0].area.width >= MIN_THUMB_WIDTH);
        assert!(eight.area.width <= 1280.0 - 2.0 * MARGIN);

        let thirty = card(
            icon(600.0, 1020.0),
            DockEdge::Bottom,
            small,
            1.0,
            &[(800.0, 600.0); 30],
        )
        .unwrap();
        assert_eq!(thirty.cells[0].area.width, MIN_THUMB_WIDTH);
        assert_eq!(thirty.cells.len() + thirty.more, 30);
        assert!(thirty.more > 0);
        assert!(thirty.area.width <= 1280.0 - 2.0 * MARGIN);
        // "+N" has its column after the last cell, inside the card.
        let more = thirty.more_area.unwrap();
        let last = thirty.cells.last().unwrap().area;
        assert!(more.x >= last.x + last.width);
        assert!(more.x + more.width <= thirty.area.width - PAD + 0.5);
        assert_eq!(eight.more_area, None);
    }

    #[test]
    fn the_sizes_grow_with_the_scale() {
        let card = card(
            icon(900.0, 1020.0),
            DockEdge::Bottom,
            SCREEN,
            1.5,
            &[(800.0, 600.0)],
        )
        .unwrap();
        assert_eq!(card.area.width, (2.0 * PAD + THUMB_WIDTH) * 1.5);
        assert_eq!(card.cells[0].close.width, CLOSE_SIDE * 1.5);
    }

    #[test]
    fn the_close_button_wins_over_its_cell() {
        let card = card(
            icon(900.0, 1020.0),
            DockEdge::Bottom,
            SCREEN,
            1.0,
            &[(800.0, 600.0); 2],
        )
        .unwrap();
        let close = card.cells[1].close;
        assert_eq!(
            hit(&card, close.x + 2.0, close.y + 2.0),
            Some(Hit::Close(1))
        );
        let thumb = card.cells[0].thumb;
        assert_eq!(hit(&card, thumb.x + 2.0, thumb.y + 2.0), Some(Hit::Cell(0)));
        assert_eq!(hit(&card, 1.0, 1.0), None);
    }

    #[test]
    fn only_several_needs_two_windows() {
        assert!(!shows(0, false));
        assert!(shows(1, false));
        assert!(!shows(1, true));
        assert!(shows(2, true));
    }
}
