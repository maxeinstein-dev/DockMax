//! The DockMax's own menus (spec 0014 R3): their lines, how the keyboard
//! moves in them, their measures and where they open. Pure: the window that
//! draws them is `windows::menu`.

/// A color as red, green and blue.
pub type Rgb = (u8, u8, u8);

/// One line of a menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Item(Item),
    Separator,
    /// A small title over the lines after it ("Espaços de trabalho").
    Title(String),
}

/// What can be chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub label: String,
    /// A glyph of Windows' icon font, before the label.
    pub glyph: Option<char>,
    /// A round swatch instead of the glyph (a workspace's color).
    pub swatch: Option<Rgb>,
    /// The keys that do the same, at the right (`Ctrl+Alt+1`).
    pub shortcut: Option<String>,
    /// It turns something on and off: checked or not, it says which.
    pub toggle: bool,
    pub checked: bool,
    pub enabled: bool,
}

impl Item {
    pub fn new(label: &str, glyph: char) -> Item {
        Item {
            label: label.to_owned(),
            glyph: Some(glyph),
            swatch: None,
            shortcut: None,
            toggle: false,
            checked: false,
            enabled: true,
        }
    }

    pub fn checked(mut self, checked: bool) -> Item {
        self.toggle = true;
        self.checked = checked;
        self
    }

    pub fn shortcut(mut self, keys: impl Into<String>) -> Item {
        self.shortcut = Some(keys.into());
        self
    }

    pub fn swatch(mut self, color: Rgb) -> Item {
        self.glyph = None;
        self.swatch = Some(color);
        self
    }
}

impl Line {
    fn choosable(&self) -> bool {
        matches!(self, Line::Item(item) if item.enabled)
    }
}

/// The item after (`forward`) or before `from`, going round, past the
/// separators, titles and disabled items; with nothing chosen yet, the
/// first (or the last). `None` when nothing can be chosen.
pub fn step(lines: &[Line], from: Option<usize>, forward: bool) -> Option<usize> {
    let count = lines.len();
    if count == 0 {
        return None;
    }
    let start = match (from, forward) {
        (Some(at), _) => at,
        (None, true) => count - 1,
        (None, false) => 0,
    };
    (1..=count)
        .map(|offset| {
            if forward {
                (start + offset) % count
            } else {
                (start + count - offset % count) % count
            }
        })
        .find(|&index| lines[index].choosable())
}

/// The first item that can be chosen.
pub fn first(lines: &[Line]) -> Option<usize> {
    step(lines, None, true)
}

/// The last item that can be chosen.
pub fn last(lines: &[Line]) -> Option<usize> {
    step(lines, None, false)
}

/// The first letter of a label, without accents and in lower case, as a
/// key press finds it ("É" and "e" both find "Encerrar").
fn initial(label: &str) -> Option<char> {
    let first = label.chars().find(|c| c.is_alphanumeric())?;
    let plain = match first.to_lowercase().next().unwrap_or(first) {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'ê' | 'è' | 'ë' => 'e',
        'í' | 'î' | 'ì' | 'ï' => 'i',
        'ó' | 'ô' | 'õ' | 'ò' | 'ö' => 'o',
        'ú' | 'û' | 'ù' | 'ü' => 'u',
        'ç' => 'c',
        other => other,
    };
    Some(plain)
}

/// What a key typed in the menu does, as in Windows' menus: the next item
/// whose label starts with that letter, after `from`; `unique` when no
/// other item starts with it (then the key chooses it at once).
pub fn by_letter(lines: &[Line], from: Option<usize>, typed: char) -> Option<(usize, bool)> {
    let wanted = initial(&typed.to_string())?;
    let matching: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.choosable())
        .filter(|(_, line)| match line {
            Line::Item(item) => initial(&item.label) == Some(wanted),
            _ => false,
        })
        .map(|(index, _)| index)
        .collect();
    let next = matching
        .iter()
        .copied()
        .find(|&index| from.is_some_and(|at| index > at))
        .or_else(|| matching.first().copied())?;
    Some((next, matching.len() == 1))
}

/// Measures, in logical pixels.
pub const PAD: f64 = 6.0;
pub const ITEM: f64 = 34.0;
pub const SEPARATOR: f64 = 9.0;
pub const TITLE: f64 = 28.0;
pub const MIN_WIDTH: f64 = 240.0;
pub const MAX_WIDTH: f64 = 420.0;
/// The gap from the side of the menu to a highlighted item.
pub const INSET: f64 = 6.0;
/// Where the label starts (after the glyph), and the gap before the
/// shortcut.
pub const LABEL_X: f64 = 44.0;
pub const SHORTCUT_GAP: f64 = 28.0;
/// The room at the right for the check.
pub const CHECK: f64 = 30.0;

fn height_of(line: &Line) -> f64 {
    match line {
        Line::Item(_) => ITEM,
        Line::Separator => SEPARATOR,
        Line::Title(_) => TITLE,
    }
}

/// The menu's height.
pub fn height(lines: &[Line]) -> f64 {
    2.0 * PAD + lines.iter().map(height_of).sum::<f64>()
}

/// Where each line starts, from the menu's top.
pub fn tops(lines: &[Line]) -> Vec<f64> {
    let mut y = PAD;
    lines
        .iter()
        .map(|line| {
            let top = y;
            y += height_of(line);
            top
        })
        .collect()
}

/// The line whose item is at `y` (from the menu's top), if it can be
/// chosen.
pub fn item_at(lines: &[Line], y: f64) -> Option<usize> {
    tops(lines)
        .into_iter()
        .zip(lines)
        .position(|(top, line)| y >= top && y < top + height_of(line))
        .filter(|&index| lines[index].choosable())
}

/// The menu's width for its widest label and shortcut (measured by the
/// caller), between the least and the most.
pub fn width(widest_label: f64, widest_shortcut: f64) -> f64 {
    let shortcut = if widest_shortcut > 0.0 {
        SHORTCUT_GAP + widest_shortcut
    } else {
        0.0
    };
    (LABEL_X + widest_label + shortcut + CHECK + INSET)
        .ceil()
        .clamp(MIN_WIDTH, MAX_WIDTH)
}

/// A box on the screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Where a menu of `width` × `height` (physical pixels) opens for a click
/// at (`x`, `y`): centered on it across, above it when `up` (the dock at
/// the bottom) and below it otherwise, flipped when it would leave `work`
/// (the monitor's work area), and always inside it.
pub fn place(x: i32, y: i32, width: i32, height: i32, up: bool, work: Area) -> Area {
    let width = width.min(work.width);
    let height = height.min(work.height);
    let left = (x - width / 2).clamp(work.x, work.x + work.width - width);
    let above = y - height;
    let below = y;
    let fits_above = above >= work.y;
    let fits_below = below + height <= work.y + work.height;
    let top = match (up, fits_above, fits_below) {
        (true, true, _) | (false, true, false) => above,
        (false, _, true) | (true, false, true) => below,
        _ => (y - height / 2).clamp(work.y, work.y + work.height - height),
    };
    Area {
        x: left,
        y: top,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> Line {
        Line::Item(Item::new(label, '\u{E713}'))
    }

    fn off(label: &str) -> Line {
        let mut item = Item::new(label, '\u{E713}');
        item.enabled = false;
        Line::Item(item)
    }

    fn menu() -> Vec<Line> {
        vec![
            item("Buscar apps…"),
            item("Gerenciador de Tarefas"),
            Line::Separator,
            Line::Title("Espaços de trabalho".into()),
            item("Principal"),
            off("Pessoal"),
            item("Novo espaço"),
            Line::Separator,
            item("Encerrar o DockMax"),
        ]
    }

    #[test]
    fn the_arrows_skip_what_cannot_be_chosen_and_go_round() {
        let lines = menu();
        assert_eq!(first(&lines), Some(0));
        assert_eq!(last(&lines), Some(8));
        assert_eq!(step(&lines, Some(1), true), Some(4));
        assert_eq!(step(&lines, Some(4), true), Some(6));
        assert_eq!(step(&lines, Some(6), false), Some(4));
        assert_eq!(step(&lines, Some(8), true), Some(0));
        assert_eq!(step(&lines, Some(0), false), Some(8));
        assert_eq!(step(&[Line::Separator], None, true), None);
        assert_eq!(step(&[], None, true), None);
    }

    #[test]
    fn a_letter_finds_the_next_item_and_chooses_a_lone_one() {
        let lines = menu();
        // Only "Gerenciador de Tarefas" starts with g: chosen at once.
        assert_eq!(by_letter(&lines, None, 'g'), Some((1, true)));
        assert_eq!(by_letter(&lines, None, 'G'), Some((1, true)));
        // "Principal" (and the disabled "Pessoal" is skipped).
        assert_eq!(by_letter(&lines, None, 'p'), Some((4, true)));
        // An accent finds the plain letter, and the other way round.
        assert_eq!(by_letter(&lines, None, 'É'), Some((8, true)));
        assert_eq!(by_letter(&lines, None, 'x'), None);
    }

    #[test]
    fn a_repeated_letter_moves_between_its_items() {
        let lines = vec![item("Nova janela"), item("Nota"), item("Abrir")];
        assert_eq!(by_letter(&lines, None, 'n'), Some((0, false)));
        assert_eq!(by_letter(&lines, Some(0), 'n'), Some((1, false)));
        assert_eq!(by_letter(&lines, Some(1), 'n'), Some((0, false)));
    }

    #[test]
    fn the_pointer_finds_only_items() {
        let lines = menu();
        let tops = tops(&lines);
        assert_eq!(tops[0], PAD);
        assert_eq!(tops[3], PAD + 2.0 * ITEM + SEPARATOR);
        assert_eq!(item_at(&lines, PAD + 1.0), Some(0));
        assert_eq!(item_at(&lines, tops[2] + 1.0), None);
        assert_eq!(item_at(&lines, tops[3] + 1.0), None);
        assert_eq!(item_at(&lines, tops[5] + 1.0), None);
        assert_eq!(item_at(&lines, 0.0), None);
        assert_eq!(
            height(&lines),
            2.0 * PAD + 6.0 * ITEM + 2.0 * SEPARATOR + TITLE
        );
    }

    #[test]
    fn the_width_fits_the_longest_line_within_bounds() {
        assert_eq!(width(10.0, 0.0), MIN_WIDTH);
        assert_eq!(width(1000.0, 0.0), MAX_WIDTH);
        assert_eq!(
            width(150.0, 60.0),
            (LABEL_X + 150.0 + SHORTCUT_GAP + 60.0 + CHECK + INSET).ceil()
        );
    }

    const WORK: Area = Area {
        x: 0,
        y: 0,
        width: 1920,
        height: 1032,
    };

    #[test]
    fn it_opens_above_a_dock_at_the_bottom_and_below_one_at_the_top() {
        let up = place(900, 1000, 300, 400, true, WORK);
        assert_eq!((up.x, up.y), (750, 600));
        let down = place(900, 60, 300, 400, false, WORK);
        assert_eq!((down.x, down.y), (750, 60));
    }

    #[test]
    fn it_flips_and_stays_on_the_screen() {
        // No room above: below.
        assert_eq!(place(900, 100, 300, 400, true, WORK).y, 100);
        // No room below: above.
        assert_eq!(place(900, 900, 300, 400, false, WORK).y, 500);
        // At the screen's side: inside it.
        assert_eq!(place(10, 1000, 300, 400, true, WORK).x, 0);
        assert_eq!(place(1915, 1000, 300, 400, true, WORK).x, 1620);
        // Taller than the room on both sides: centered on the click, inside.
        let tall = place(900, 500, 300, 900, true, WORK);
        assert!(tall.y >= 0 && tall.y + tall.height <= 1032);
    }
}
