//! The few SVG path commands the dock's glyphs use (`src/dock/Glyph.tsx`,
//! the note's tile, the tile's superellipse), read into absolute segments
//! Direct2D can draw: M, L, H, V, C, S, A and Z, absolute and relative.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Move(f32, f32),
    Line(f32, f32),
    Cubic([f32; 6]),
    Arc {
        rx: f32,
        ry: f32,
        rotation: f32,
        large: bool,
        sweep: bool,
        x: f32,
        y: f32,
    },
    Close,
}

/// Reads the numbers of a path, as SVG allows them to be written: separated
/// by spaces or commas, or run together ("1.5.5", "2-1"), and the arc flags
/// as single digits ("0 1 0" or "010").
struct Numbers<'a> {
    text: &'a [u8],
    at: usize,
}

impl Numbers<'_> {
    fn skip(&mut self) {
        while self.at < self.text.len() && matches!(self.text[self.at], b' ' | b',' | b'\n' | b'\t')
        {
            self.at += 1;
        }
    }

    /// Whether a number comes next (and not a command).
    fn more(&mut self) -> bool {
        self.skip();
        self.text
            .get(self.at)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Option<f32> {
        self.skip();
        let start = self.at;
        if matches!(self.text.get(self.at), Some(b'-' | b'+')) {
            self.at += 1;
        }
        let mut dot = false;
        while let Some(&c) = self.text.get(self.at) {
            if c.is_ascii_digit() {
                self.at += 1;
            } else if c == b'.' && !dot {
                dot = true;
                self.at += 1;
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.text[start..self.at])
            .ok()?
            .parse()
            .ok()
    }

    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let flag = match self.text.get(self.at)? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.at += 1;
        Some(flag)
    }
}

/// The segments of `path`; `None` when it is not a path this reader knows.
pub fn parse(path: &str) -> Option<Vec<Segment>> {
    let mut numbers = Numbers {
        text: path.as_bytes(),
        at: 0,
    };
    let mut segments = Vec::new();
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let (mut start_x, mut start_y) = (0.0f32, 0.0f32);
    // The second control point of the last cubic, for S.
    let mut last_control: Option<(f32, f32)> = None;
    loop {
        numbers.skip();
        let Some(&command) = numbers.text.get(numbers.at) else {
            break;
        };
        numbers.at += 1;
        let relative = command.is_ascii_lowercase();
        let origin = |relative: bool, x: f32, y: f32| if relative { (x, y) } else { (0.0, 0.0) };
        let mut first = true;
        loop {
            if !first && !numbers.more() {
                break;
            }
            match command.to_ascii_uppercase() {
                b'M' => {
                    let (ox, oy) = origin(relative, x, y);
                    x = ox + numbers.number()?;
                    y = oy + numbers.number()?;
                    if first {
                        segments.push(Segment::Move(x, y));
                        start_x = x;
                        start_y = y;
                    } else {
                        // Pairs after the first of an M are lines.
                        segments.push(Segment::Line(x, y));
                    }
                    last_control = None;
                }
                b'L' => {
                    let (ox, oy) = origin(relative, x, y);
                    x = ox + numbers.number()?;
                    y = oy + numbers.number()?;
                    segments.push(Segment::Line(x, y));
                    last_control = None;
                }
                b'H' => {
                    let (ox, _) = origin(relative, x, y);
                    x = ox + numbers.number()?;
                    segments.push(Segment::Line(x, y));
                    last_control = None;
                }
                b'V' => {
                    let (_, oy) = origin(relative, x, y);
                    y = oy + numbers.number()?;
                    segments.push(Segment::Line(x, y));
                    last_control = None;
                }
                b'C' | b'S' => {
                    let (ox, oy) = origin(relative, x, y);
                    let (c1x, c1y) = if command.eq_ignore_ascii_case(&b'C') {
                        (ox + numbers.number()?, oy + numbers.number()?)
                    } else {
                        // The reflection of the last control point.
                        last_control.map_or((x, y), |(cx, cy)| (2.0 * x - cx, 2.0 * y - cy))
                    };
                    let c2x = ox + numbers.number()?;
                    let c2y = oy + numbers.number()?;
                    x = ox + numbers.number()?;
                    y = oy + numbers.number()?;
                    segments.push(Segment::Cubic([c1x, c1y, c2x, c2y, x, y]));
                    last_control = Some((c2x, c2y));
                }
                b'A' => {
                    let (ox, oy) = origin(relative, x, y);
                    let rx = numbers.number()?;
                    let ry = numbers.number()?;
                    let rotation = numbers.number()?;
                    let large = numbers.flag()?;
                    let sweep = numbers.flag()?;
                    x = ox + numbers.number()?;
                    y = oy + numbers.number()?;
                    segments.push(Segment::Arc {
                        rx,
                        ry,
                        rotation,
                        large,
                        sweep,
                        x,
                        y,
                    });
                    last_control = None;
                }
                b'Z' => {
                    segments.push(Segment::Close);
                    x = start_x;
                    y = start_y;
                    last_control = None;
                    break;
                }
                _ => return None,
            }
            first = false;
        }
    }
    Some(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_with_relative_and_absolute_commands() {
        assert_eq!(
            parse("M4 7.5h9.5M17.5 7.5H20").unwrap(),
            vec![
                Segment::Move(4.0, 7.5),
                Segment::Line(13.5, 7.5),
                Segment::Move(17.5, 7.5),
                Segment::Line(20.0, 7.5),
            ]
        );
        assert_eq!(
            parse("M6 9.5l6 6 6-6").unwrap(),
            vec![
                Segment::Move(6.0, 9.5),
                Segment::Line(12.0, 15.5),
                Segment::Line(18.0, 9.5),
            ]
        );
        assert_eq!(
            parse("M12 11v5.5").unwrap(),
            vec![Segment::Move(12.0, 11.0), Segment::Line(12.0, 16.5)]
        );
    }

    #[test]
    fn arcs_with_flags_run_together_or_apart() {
        let segments = parse("M13.5 7.5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0").unwrap();
        assert_eq!(
            segments[1],
            Segment::Arc {
                rx: 2.0,
                ry: 2.0,
                rotation: 0.0,
                large: true,
                sweep: false,
                x: 17.5,
                y: 7.5,
            }
        );
        assert_eq!(
            segments[2],
            Segment::Arc {
                rx: 2.0,
                ry: 2.0,
                rotation: 0.0,
                large: true,
                sweep: false,
                x: 13.5,
                y: 7.5,
            }
        );
        let packed = parse("M0 0a1.5 1.5 0 0 1 1.5 1.5").unwrap();
        assert_eq!(
            packed[1],
            Segment::Arc {
                rx: 1.5,
                ry: 1.5,
                rotation: 0.0,
                large: false,
                sweep: true,
                x: 1.5,
                y: 1.5,
            }
        );
    }

    #[test]
    fn the_superellipse_of_the_tile_with_smooth_curves() {
        let segments =
            parse("M50 0C90 0 100 10 100 50S90 100 50 100 0 90 0 50 10 0 50 0Z").unwrap();

        assert_eq!(segments[0], Segment::Move(50.0, 0.0));
        assert_eq!(
            segments[1],
            Segment::Cubic([90.0, 0.0, 100.0, 10.0, 100.0, 50.0])
        );
        // S reflects the last control point (100 10) about (100 50).
        assert_eq!(
            segments[2],
            Segment::Cubic([100.0, 90.0, 90.0, 100.0, 50.0, 100.0])
        );
        assert_eq!(
            segments[3],
            Segment::Cubic([10.0, 100.0, 0.0, 90.0, 0.0, 50.0])
        );
        assert_eq!(
            segments[4],
            Segment::Cubic([0.0, 10.0, 10.0, 0.0, 50.0, 0.0])
        );
        assert_eq!(segments[5], Segment::Close);
    }

    #[test]
    fn a_closed_shape_and_a_dot() {
        let segments = parse(
            "M4.5 5h15a1.5 1.5 0 0 1 1.5 1.5v11a1.5 1.5 0 0 1-1.5 1.5h-15a1.5 1.5 0 0 1-1.5-1.5v-11A1.5 1.5 0 0 1 4.5 5z",
        )
        .unwrap();
        assert_eq!(segments.last(), Some(&Segment::Close));
        assert_eq!(segments.len(), 10);
        assert_eq!(parse("M12 7.6h.01").unwrap()[1], Segment::Line(12.01, 7.6));
    }

    #[test]
    fn unknown_commands_are_refused() {
        assert_eq!(parse("M0 0Q1 1 2 2"), None);
        assert_eq!(parse("M0"), None);
    }
}
