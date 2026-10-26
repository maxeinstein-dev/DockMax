//! The dock's geometry, computed instead of measured (spec 0011 H3): where
//! the bar, the items and their tiles sit in the dock window, how much each
//! item grows under the pointer, the window the dock needs and where a
//! dragged program would land. The same rules the page's dock followed
//! (`src/dock/layout.ts`, `magnification.ts` and `drag.ts`), with the same
//! numbers, in logical pixels:
//!
//! - the bar is `icon + 2 × pad` tall, flush with the window edge that faces
//!   the screen edge and centered across the window;
//! - each item is `icon × scale` wide (the magnification pushes its
//!   neighbours apart), `gap` apart, `pad` inside the bar;
//! - a tile grows from the middle of its screen-side edge and moves `lift`
//!   away from it;
//! - the label pill sits `TIP_GAP` beyond the tile, on the far side.

use super::settings::{DockEdge, HoverEffect, SizePreset};

/// A rectangle in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Area {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }

    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }
}

/// The size presets the dock draws (DESIGN.md "Layout"): every icon size of
/// the settings is drawn as the nearest one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSize {
    Small,
    Medium,
    Large,
}

/// Icon, padding and gap of a size preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeGeometry {
    pub icon: f64,
    pub pad: f64,
    pub gap: f64,
}

impl DockSize {
    /// The preset closest to `icon_size`; a tie goes to the smaller one.
    pub fn of(icon_size: u16) -> DockSize {
        let presets = [
            (SizePreset::Small, DockSize::Small),
            (SizePreset::Medium, DockSize::Medium),
            (SizePreset::Large, DockSize::Large),
        ];
        let mut best = DockSize::Medium;
        let mut best_distance = u16::MAX;
        for (preset, size) in presets {
            let distance = preset.icon_size().abs_diff(icon_size);
            if distance < best_distance {
                best = size;
                best_distance = distance;
            }
        }
        best
    }

    pub const fn geometry(self) -> SizeGeometry {
        match self {
            DockSize::Small => SizeGeometry {
                icon: 40.0,
                pad: 6.0,
                gap: 6.0,
            },
            DockSize::Medium => SizeGeometry {
                icon: 48.0,
                pad: 7.0,
                gap: 7.0,
            },
            DockSize::Large => SizeGeometry {
                icon: 64.0,
                pad: 9.0,
                gap: 9.0,
            },
        }
    }
}

/// Distance between a tile and its label pill (DESIGN.md "Tooltip").
pub const TIP_GAP: f64 = 10.0;

/// Height of the label pill: a 16 px line plus 4 px above and below.
pub const TIP_HEIGHT: f64 = 24.0;

/// Width of the clock and date at the end of the bar.
pub const STATUS_WIDTH: f64 = 76.0;

/// Transparent room the window keeps beyond the label and on both sides of
/// the widest bar, for the pill's shadow and the bar's.
pub const ROOM: f64 = 8.0;

/// Width a separator takes between two groups, its margins included (a 1 px
/// line with `gap × 0.4` on each side), besides the gap that follows it.
pub fn separator_width(gap: f64) -> f64 {
    1.0 + gap * 0.8
}

/// How much an item grows: its tile is scaled by `scale` from the middle of
/// its screen-side edge, then moved `lift` away from it. The item's box
/// widens by the same `scale`, so the growth pushes the neighbours apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Magnification {
    pub scale: f64,
    pub lift: f64,
}

const REST: Magnification = Magnification {
    scale: 1.0,
    lift: 0.0,
};

/// The "magnify" effect (DESIGN.md "Layout", Ampliação B1): the hovered item
/// and each direct neighbour.
pub const MAGNIFY_HOVERED: Magnification = Magnification {
    scale: 1.4,
    lift: 6.0,
};
pub const MAGNIFY_NEIGHBOR: Magnification = Magnification {
    scale: 1.15,
    lift: 0.0,
};

/// What an item does under the pointer, and how much the "scale" effect
/// grows it (a ratio: 1.1 is 110 %).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hover {
    pub effect: HoverEffect,
    pub scale: f64,
}

impl Hover {
    pub fn of(effect: HoverEffect, percent: u16) -> Hover {
        Hover {
            effect,
            scale: f64::from(percent) / 100.0,
        }
    }

    /// The growth of the hovered item at its peak: what the window must
    /// leave room for.
    pub fn peak(self) -> Magnification {
        match self.effect {
            HoverEffect::None => REST,
            HoverEffect::Scale => Magnification {
                scale: self.scale,
                lift: 0.0,
            },
            HoverEffect::Magnify => MAGNIFY_HOVERED,
        }
    }

    /// The growth of the item at `index` while the item at `magnified` is
    /// hovered.
    pub fn of_item(self, index: usize, magnified: Option<usize>) -> Magnification {
        let Some(magnified) = magnified else {
            return REST;
        };
        match self.effect {
            HoverEffect::None => REST,
            _ if index == magnified => self.peak(),
            HoverEffect::Scale => REST,
            HoverEffect::Magnify if index.abs_diff(magnified) == 1 => MAGNIFY_NEIGHBOR,
            HoverEffect::Magnify => REST,
        }
    }
}

/// What the dock shows: its size, the number of items in each group (a
/// separator stands between groups that have items), what an item does
/// under the pointer and whether the clock ends the bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub size: DockSize,
    pub groups: Vec<usize>,
    pub hover: Hover,
    pub status: bool,
}

/// One item along the bar: its left edge from the bar's outer left edge,
/// its width and its growth.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowItem {
    x: f64,
    width: f64,
    magnification: Magnification,
}

/// Lays the items out along the bar with each item at `scales` (as it is
/// now, mid-animation included): the bar's width, each item and where the
/// separators and the clock sit (from the bar's outer left edge).
#[derive(Debug, Clone, PartialEq)]
struct Row {
    width: f64,
    items: Vec<RowItem>,
    separators: Vec<f64>,
    status: Option<f64>,
}

fn row(shape: &Shape, growth: &dyn Fn(usize) -> Magnification) -> Row {
    let SizeGeometry { icon, pad, gap } = shape.size.geometry();
    let mut items = Vec::new();
    let mut separators = Vec::new();
    let mut x = pad;
    for &count in shape.groups.iter().filter(|&&count| count > 0) {
        if !items.is_empty() {
            separators.push(x + gap * 0.4);
            x += separator_width(gap) + gap;
        }
        for _ in 0..count {
            let magnification = growth(items.len());
            let width = icon * magnification.scale;
            items.push(RowItem {
                x,
                width,
                magnification,
            });
            x += width + gap;
        }
    }
    let mut status = None;
    if shape.status {
        // [item][gap][separator][gap][status][gap]: the way two groups meet.
        if !items.is_empty() {
            separators.push(x + gap * 0.4);
            x += separator_width(gap) + gap;
        }
        status = Some(x);
        x += STATUS_WIDTH + gap;
    }
    let width = if items.is_empty() && !shape.status {
        2.0 * pad
    } else {
        x - gap + pad
    };
    Row {
        width,
        items,
        separators,
        status,
    }
}

/// Rounds up to whole pixels, ignoring floating-point noise.
fn ceil(value: f64) -> f64 {
    (value - 1e-6).ceil()
}

/// The window the dock needs, whatever is magnified or labelled: the bar,
/// the magnified tile and the label pill beyond it, plus `ROOM`; and,
/// across, the widest the bar gets or the farthest a label reaches past it,
/// plus `ROOM` on each side. `label_widths` are each item's pill.
pub fn window_size(shape: &Shape, motion: bool, label_widths: &[f64]) -> (f64, f64) {
    let SizeGeometry { icon, pad, .. } = shape.size.geometry();
    let peak = shape.hover.peak();
    let grows = motion && peak.scale != 1.0;
    let Magnification { scale, lift } = if grows { peak } else { REST };
    let height = pad + icon * scale + lift + TIP_GAP + TIP_HEIGHT + ROOM;

    let count = row(shape, &|_| REST).items.len();
    let mut states = vec![None];
    if grows {
        states.extend((0..count).map(Some));
    }
    let mut half: f64 = 0.0;
    for state in states {
        let Row { width, items, .. } = row(shape, &|index| shape.hover.of_item(index, state));
        half = half.max(width / 2.0);
        for (index, item) in items.iter().enumerate() {
            if state.is_some_and(|magnified| magnified != index) {
                continue;
            }
            let center = item.x + item.width / 2.0 - width / 2.0;
            let label = label_widths.get(index).copied().unwrap_or(0.0);
            half = half.max(center.abs() + label / 2.0);
        }
    }
    (ceil(2.0 * half + 2.0 * ROOM), ceil(height))
}

/// Where everything sits in a window of `viewport`.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// The visible bar.
    pub bar: Area,
    /// Each item's box.
    pub items: Vec<Area>,
    /// Each item's tile, grown.
    pub tiles: Vec<Area>,
    /// Each item's scale.
    pub scales: Vec<f64>,
    /// The x of each separator's line.
    pub separators: Vec<f64>,
    /// The clock's box, when it ends the bar.
    pub status: Option<Area>,
}

/// Where the bar, the items and the tiles are when each item has grown as
/// `growth` says.
pub fn place_with(
    shape: &Shape,
    edge: DockEdge,
    viewport: (f64, f64),
    growth: &dyn Fn(usize) -> Magnification,
) -> Placement {
    let SizeGeometry { icon, pad, .. } = shape.size.geometry();
    let row = row(shape, growth);
    let height = icon + 2.0 * pad;
    let bar = Area {
        x: (viewport.0 - row.width) / 2.0,
        y: match edge {
            DockEdge::Bottom => viewport.1 - height,
            DockEdge::Top => 0.0,
        },
        width: row.width,
        height,
    };
    let top = bar.y + pad;
    Placement {
        bar,
        items: row
            .items
            .iter()
            .map(|item| Area {
                x: bar.x + item.x,
                y: top,
                width: item.width,
                height: icon,
            })
            .collect(),
        tiles: row
            .items
            .iter()
            .map(|item| Area {
                x: bar.x + item.x,
                y: match edge {
                    DockEdge::Bottom => top + icon - item.magnification.lift - item.width,
                    DockEdge::Top => top + item.magnification.lift,
                },
                width: item.width,
                height: item.width,
            })
            .collect(),
        scales: row
            .items
            .iter()
            .map(|item| item.magnification.scale)
            .collect(),
        separators: row.separators.iter().map(|x| bar.x + x).collect(),
        status: row.status.map(|x| Area {
            x: bar.x + x,
            y: top,
            width: STATUS_WIDTH,
            height: icon,
        }),
    }
}

/// Where everything ends up once the magnification of `magnified` settles.
pub fn place(
    shape: &Shape,
    edge: DockEdge,
    viewport: (f64, f64),
    magnified: Option<usize>,
) -> Placement {
    place_with(shape, edge, viewport, &|index| {
        shape.hover.of_item(index, magnified)
    })
}

/// Where the label pill of `tile` is anchored: the middle of its edge nearest
/// the tile, `TIP_GAP` beyond the tile on the far side.
pub fn tip_anchor(tile: Area, edge: DockEdge) -> (f64, f64) {
    (
        tile.center_x(),
        match edge {
            DockEdge::Bottom => tile.y - TIP_GAP,
            DockEdge::Top => tile.y + tile.height + TIP_GAP,
        },
    )
}

/// How far the pointer goes before a press becomes a drag: a click that
/// moves a little is still a click.
pub const DRAG_THRESHOLD: f64 = 6.0;

/// Where a pinned program would go, among the pinned programs, if the
/// pointer were let go at `x`: the number of the other programs whose centre
/// is before it.
pub fn drop_index(centers: &[f64], from: usize, x: f64) -> usize {
    centers
        .iter()
        .enumerate()
        .filter(|&(index, &center)| index != from && center < x)
        .count()
}

/// Which side of which program the mark of the drop stands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSide {
    Before,
    After,
}

/// Where the mark of the drop stands, or nowhere when the program would stay
/// where it is.
pub fn drop_mark(count: usize, from: usize, target: usize) -> Option<(usize, DropSide)> {
    if target == from {
        return None;
    }
    let others: Vec<usize> = (0..count).filter(|&index| index != from).collect();
    match others.get(target) {
        Some(&next) => Some((next, DropSide::Before)),
        None => others.last().map(|&last| (last, DropSide::After)),
    }
}

/// Keeps the dragged tile inside the bar: `x` is the pointer, `start` where
/// the drag began, and the tile can go from `min` to `max`.
pub fn drag_offset(x: f64, start: f64, rest_left: f64, width: f64, min: f64, max: f64) -> f64 {
    (x - start)
        .max(min - rest_left)
        .min(max - width - rest_left)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Configurações and Sobre, one group, with the Fase 1 magnification.
    fn shape(size: DockSize) -> Shape {
        Shape {
            size,
            groups: vec![2],
            hover: Hover {
                effect: HoverEffect::Magnify,
                scale: 1.4,
            },
            status: false,
        }
    }

    const LABELS: [f64; 2] = [110.0, 60.0];

    fn area(x: f64, y: f64, width: f64, height: f64) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    fn close(actual: Area, expected: Area) {
        for (a, e) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.width, expected.width),
            (actual.height, expected.height),
        ] {
            assert!((a - e).abs() < 1e-6, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn icon_sizes_draw_the_nearest_preset() {
        assert_eq!(DockSize::of(32), DockSize::Small);
        assert_eq!(DockSize::of(40), DockSize::Small);
        assert_eq!(DockSize::of(44), DockSize::Small);
        assert_eq!(DockSize::of(45), DockSize::Medium);
        assert_eq!(DockSize::of(56), DockSize::Medium);
        assert_eq!(DockSize::of(57), DockSize::Large);
        assert_eq!(DockSize::of(96), DockSize::Large);
    }

    #[test]
    fn sizes_match_the_presets_of_the_design() {
        let geometry = |size: DockSize| {
            let SizeGeometry { icon, pad, gap } = size.geometry();
            (icon, pad, gap)
        };
        assert_eq!(geometry(DockSize::Small), (40.0, 6.0, 6.0));
        assert_eq!(geometry(DockSize::Medium), (48.0, 7.0, 7.0));
        assert_eq!(geometry(DockSize::Large), (64.0, 9.0, 9.0));
    }

    #[test]
    fn window_fits_each_size_magnified_with_its_labels() {
        for (size, expected) in [
            (DockSize::Small, (178.0, 110.0)),
            (DockSize::Medium, (189.0, 123.0)),
            (DockSize::Large, (209.0, 147.0)),
        ] {
            assert_eq!(window_size(&shape(size), true, &LABELS), expected);
        }
    }

    #[test]
    fn window_fits_each_size_at_rest_with_reduced_motion() {
        for (size, expected) in [
            (DockSize::Small, (172.0, 88.0)),
            (DockSize::Medium, (181.0, 97.0)),
            (DockSize::Large, (199.0, 115.0)),
        ] {
            assert_eq!(window_size(&shape(size), false, &LABELS), expected);
        }
    }

    #[test]
    fn window_fits_the_widest_bar_when_the_labels_are_narrow() {
        assert_eq!(
            window_size(&shape(DockSize::Medium), true, &[0.0, 0.0]),
            (160.0, 123.0)
        );
    }

    #[test]
    fn window_grows_with_a_long_label() {
        let narrow = window_size(&shape(DockSize::Medium), true, &LABELS);
        let wide = window_size(&shape(DockSize::Medium), true, &[200.0, 60.0]);

        assert_eq!(wide.0 - narrow.0, 90.0);
        assert_eq!(wide.1, narrow.1);
    }

    #[test]
    fn the_bar_at_rest_sits_on_the_bottom_edge_centered() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (189.0, 123.0),
            None,
        );

        close(placement.bar, area(36.0, 61.0, 117.0, 62.0));
        close(placement.items[0], area(43.0, 68.0, 48.0, 48.0));
        close(placement.items[1], area(98.0, 68.0, 48.0, 48.0));
        close(placement.tiles[1], area(98.0, 68.0, 48.0, 48.0));
        assert_eq!(placement.scales, vec![1.0, 1.0]);
    }

    #[test]
    fn magnifying_pushes_the_neighbour_apart_and_lifts_the_tile() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (189.0, 123.0),
            Some(0),
        );

        close(placement.bar, area(22.8, 61.0, 143.4, 62.0));
        close(placement.items[0], area(29.8, 68.0, 67.2, 48.0));
        close(placement.items[1], area(104.0, 68.0, 55.2, 48.0));
        close(placement.tiles[0], area(29.8, 42.8, 67.2, 67.2));
        close(placement.tiles[1], area(104.0, 60.8, 55.2, 55.2));
        assert_eq!(placement.scales, vec![1.4, 1.15]);
    }

    #[test]
    fn everything_is_mirrored_on_the_top_edge() {
        let placement = place(
            &shape(DockSize::Medium),
            DockEdge::Top,
            (189.0, 123.0),
            Some(0),
        );

        close(placement.bar, area(22.8, 0.0, 143.4, 62.0));
        close(placement.tiles[0], area(29.8, 13.0, 67.2, 67.2));
        close(placement.tiles[1], area(104.0, 7.0, 55.2, 55.2));
    }

    #[test]
    fn a_separator_stands_between_groups() {
        let shape = Shape {
            groups: vec![2, 1],
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (400.0, 123.0), None);

        let third = 7.0 + 2.0 * (48.0 + 7.0) + separator_width(7.0) + 7.0;
        assert!((separator_width(7.0) - 6.6).abs() < 1e-9);
        assert!((placement.bar.width - (third + 48.0 + 7.0)).abs() < 1e-9);
        assert!((placement.items[2].x - (placement.bar.x + third)).abs() < 1e-9);
        assert_eq!(placement.separators.len(), 1);
        // The line is in the middle of its room, between the two gaps.
        let line = placement.separators[0];
        let second_end = placement.items[1].x + placement.items[1].width;
        assert!((line - (second_end + 7.0 + 2.8)).abs() < 1e-9);
    }

    #[test]
    fn empty_groups_add_no_separator() {
        let shape = Shape {
            groups: vec![0, 1, 0, 2],
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (400.0, 123.0), None);

        assert_eq!(placement.items.len(), 3);
        assert_eq!(placement.separators.len(), 1);
    }

    #[test]
    fn the_clock_adds_a_separator_and_its_width_and_moves_no_item() {
        let without = place(
            &shape(DockSize::Medium),
            DockEdge::Bottom,
            (400.0, 123.0),
            Some(0),
        );
        let with = place(
            &Shape {
                status: true,
                ..shape(DockSize::Medium)
            },
            DockEdge::Bottom,
            (400.0, 123.0),
            Some(0),
        );

        let added = with.bar.width - without.bar.width;
        assert!((added - (2.0 * 7.0 + separator_width(7.0) + STATUS_WIDTH)).abs() < 1e-9);
        for (a, b) in without.items.iter().zip(&with.items) {
            assert!(((a.x - without.bar.x) - (b.x - with.bar.x)).abs() < 1e-9);
        }
        let status = with.status.expect("the clock is placed");
        assert!((status.x + STATUS_WIDTH + 7.0 - (with.bar.x + with.bar.width)).abs() < 1e-9);
    }

    #[test]
    fn the_clock_alone_is_a_bar_of_its_own() {
        let shape = Shape {
            groups: vec![],
            status: true,
            ..shape(DockSize::Medium)
        };
        let placement = place(&shape, DockEdge::Bottom, (300.0, 100.0), None);

        assert!((placement.bar.width - (14.0 + STATUS_WIDTH)).abs() < 1e-9);
    }

    #[test]
    fn the_label_hangs_beyond_the_tile() {
        let tile = area(29.8, 42.8, 67.2, 67.2);
        let (x, y) = tip_anchor(tile, DockEdge::Bottom);
        assert!((x - 63.4).abs() < 1e-9 && (y - 32.8).abs() < 1e-9);
        let (_, y) = tip_anchor(area(29.8, 13.0, 67.2, 67.2), DockEdge::Top);
        assert!((y - 90.2).abs() < 1e-9);
    }

    #[test]
    fn the_window_fits_everything_the_dock_paints() {
        for size in [DockSize::Small, DockSize::Medium, DockSize::Large] {
            for edge in [DockEdge::Bottom, DockEdge::Top] {
                for motion in [true, false] {
                    let shape = shape(size);
                    let window = window_size(&shape, motion, &LABELS);
                    let states: &[Option<usize>] = if motion {
                        &[None, Some(0), Some(1)]
                    } else {
                        &[None]
                    };
                    let within = |inner: Area| {
                        inner.x >= -1e-9
                            && inner.y >= -1e-9
                            && inner.x + inner.width <= window.0 + 1e-9
                            && inner.y + inner.height <= window.1 + 1e-9
                    };
                    for &magnified in states {
                        let placement = place(&shape, edge, window, magnified);
                        assert!(within(placement.bar));
                        for (index, tile) in placement.tiles.iter().enumerate() {
                            assert!(within(*tile));
                            if magnified.is_some_and(|m| m != index) {
                                continue;
                            }
                            let (x, y) = tip_anchor(*tile, edge);
                            let width = LABELS[index];
                            let pill_y = match edge {
                                DockEdge::Bottom => y - TIP_HEIGHT - ROOM,
                                DockEdge::Top => y,
                            };
                            assert!(
                                within(area(
                                    x - width / 2.0 - ROOM,
                                    pill_y,
                                    width + 2.0 * ROOM,
                                    TIP_HEIGHT + ROOM,
                                )),
                                "{size:?} {edge:?} {motion} item {index} magnified {magnified:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hover_effects_grow_the_items_they_say() {
        let scale = Hover::of(HoverEffect::Scale, 120);
        let magnify = Hover::of(HoverEffect::Magnify, 120);
        let none = Hover::of(HoverEffect::None, 120);

        assert_eq!(scale.of_item(1, Some(1)).scale, 1.2);
        assert_eq!(scale.of_item(0, Some(1)), REST);
        assert_eq!(magnify.of_item(1, Some(1)), MAGNIFY_HOVERED);
        assert_eq!(magnify.of_item(0, Some(1)), MAGNIFY_NEIGHBOR);
        assert_eq!(magnify.of_item(2, Some(1)), MAGNIFY_NEIGHBOR);
        assert_eq!(magnify.of_item(3, Some(1)), REST);
        assert_eq!(none.of_item(1, Some(1)), REST);
        assert_eq!(magnify.of_item(1, None), REST);
        assert_eq!(none.peak(), REST);
    }

    #[test]
    fn a_drag_lands_after_the_programs_whose_centre_it_passed() {
        let centers = [10.0, 50.0, 90.0, 130.0];

        assert_eq!(drop_index(&centers, 0, 5.0), 0);
        assert_eq!(drop_index(&centers, 0, 60.0), 1);
        assert_eq!(drop_index(&centers, 0, 200.0), 3);
        assert_eq!(drop_index(&centers, 3, 0.0), 0);
        assert_eq!(drop_index(&centers, 1, 60.0), 1);
    }

    #[test]
    fn the_drop_mark_stands_before_the_next_program_or_after_the_last() {
        assert_eq!(drop_mark(4, 1, 1), None);
        assert_eq!(drop_mark(4, 0, 2), Some((3, DropSide::Before)));
        assert_eq!(drop_mark(4, 3, 0), Some((0, DropSide::Before)));
        assert_eq!(drop_mark(4, 0, 3), Some((3, DropSide::After)));
        assert_eq!(drop_mark(1, 0, 1), None);
    }

    #[test]
    fn the_dragged_tile_stays_inside_the_bar() {
        // Rest at 50, 40 wide, the programs from 10 to 200.
        assert_eq!(drag_offset(100.0, 70.0, 50.0, 40.0, 10.0, 200.0), 30.0);
        assert_eq!(drag_offset(0.0, 70.0, 50.0, 40.0, 10.0, 200.0), -40.0);
        assert_eq!(drag_offset(500.0, 70.0, 50.0, 40.0, 10.0, 200.0), 110.0);
    }
}
