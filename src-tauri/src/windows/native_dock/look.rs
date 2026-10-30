//! The styles of the bar (spec 0005 §3.2): each style's own look, the
//! user's changes to it, and its colors (the blocks of `src/dock/Dock.css`
//! and `src/dock/look.ts` the page's dock read).

use super::theme::{Color, Palette};
use crate::domain::settings::{DockSettings, DockStyle};

/// How the bar is filled: one color, or bands of a vertical gradient (from
/// the top), drawn over a base color.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Color),
    Gradient {
        base: Option<Color>,
        stops: Vec<(f32, Color)>,
        /// A second gradient over the first.
        over: Vec<(f32, Color)>,
    },
}

impl Fill {
    /// Whether nothing is drawn (the bar is see-through): a breath of alpha
    /// keeps the mouse on it anyway.
    pub fn is_clear(&self) -> bool {
        matches!(self, Fill::Solid(color) if color.a < 0.01)
    }
}

/// A soft light around the bar: its color and how far it reaches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glow {
    pub color: Color,
    pub reach: f32,
}

/// Everything the bar's paint reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub style: DockStyle,
    pub border: bool,
    /// How round the bar is, 0–1 of a full pill.
    pub corners: f64,
    pub reflections: bool,
    /// The style's own color of the mark under an item, or the accent.
    pub indicator: Option<Color>,
    pub fill: Option<Fill>,
    pub separator: Option<Color>,
    /// The ink of the clock at the end of the bar.
    pub ink: Option<Color>,
    /// The light edge; `Some(transparent)` takes it away.
    pub hi: Option<Color>,
    pub glows: Vec<Glow>,
    /// The bar draws no shadow (the floating styles).
    pub flat: bool,
    /// A gradient outline instead of the hairline (Neon).
    pub outline: Option<(Color, Color)>,
    /// The silver shelf under the tiles (Prateleira Aqua).
    pub shelf: bool,
    /// The bar is the style's skin (spec 0020 §3): its image, not a fill,
    /// an edge or a border.
    pub skin: bool,
}

/// The parts of a style the user can change, as each style has them.
fn own(style: DockStyle) -> (bool, u8, bool) {
    match style {
        DockStyle::Native => (true, 65, false),
        DockStyle::LiquidGlass => (true, 100, false),
        DockStyle::Midnight => (true, 25, false),
        DockStyle::Rocket => (true, 25, false),
        DockStyle::AquaShelf => (false, 0, true),
        DockStyle::Neon => (true, 70, false),
        DockStyle::Paper => (false, 40, false),
        DockStyle::Floating => (false, 40, true),
        // The image is the bar: no border and no corners to change.
        DockStyle::Retro | DockStyle::Classic98 | DockStyle::Phosphor | DockStyle::Tactical => {
            (false, 0, false)
        }
    }
}

const WHITE: Color = Color::WHITE;

impl Look {
    pub fn of(settings: &DockSettings) -> Look {
        let style = settings.style;
        let tweaks = settings.tweaks.get(&style);
        let (border, corners, reflections) = own(style);
        let base = Look {
            style,
            border: tweaks.and_then(|t| t.border).unwrap_or(border),
            corners: f64::from(tweaks.and_then(|t| t.corners).unwrap_or(corners).min(100)) / 100.0,
            reflections: tweaks.and_then(|t| t.reflections).unwrap_or(reflections),
            indicator: None,
            fill: None,
            separator: None,
            ink: None,
            hi: None,
            glows: Vec::new(),
            flat: false,
            outline: None,
            shelf: false,
            skin: false,
        };
        match style {
            DockStyle::Native => base,
            DockStyle::LiquidGlass => Look {
                fill: Some(Fill::Gradient {
                    base: Some(Color::rgba(32, 38, 56, 0.5)),
                    stops: vec![(0.0, WHITE.with_alpha(0.24)), (1.0, WHITE.with_alpha(0.07))],
                    over: Vec::new(),
                }),
                separator: Some(WHITE.with_alpha(0.28)),
                ink: Some(WHITE),
                glows: vec![Glow {
                    color: WHITE.with_alpha(0.18),
                    reach: 6.0,
                }],
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::Midnight => Look {
                fill: Some(Fill::Solid(Color::rgba(8, 10, 16, 0.93))),
                separator: Some(WHITE.with_alpha(0.2)),
                ink: Some(WHITE),
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::Rocket => Look {
                fill: Some(Fill::Solid(Color::rgba(9, 14, 32, 0.95))),
                separator: Some(Color::rgba(120, 160, 255, 0.3)),
                ink: Some(WHITE),
                glows: vec![Glow {
                    color: Color::rgba(80, 130, 255, 0.6),
                    reach: 7.0,
                }],
                indicator: Some(Color::rgb(0x8f, 0xb0, 0xff)),
                ..base
            },
            DockStyle::Neon => Look {
                fill: Some(Fill::Solid(Color::rgba(6, 6, 12, 0.96))),
                separator: Some(WHITE.with_alpha(0.2)),
                ink: Some(WHITE),
                glows: vec![
                    Glow {
                        color: Color::rgba(0, 229, 255, 0.55),
                        reach: 6.0,
                    },
                    Glow {
                        color: Color::rgba(255, 43, 214, 0.35),
                        reach: 7.0,
                    },
                ],
                outline: base
                    .border
                    .then_some((Color::rgb(0x00, 0xe5, 0xff), Color::rgb(0xff, 0x2b, 0xd6))),
                indicator: Some(Color::rgb(0x00, 0xe5, 0xff)),
                ..base
            },
            DockStyle::Paper => Look {
                fill: Some(Fill::Solid(Color::rgb(247, 247, 243))),
                separator: Some(Color::rgba(0, 0, 0, 0.14)),
                ink: Some(Color::rgb(0x1d, 0x1e, 0x22)),
                hi: Some(Color::CLEAR),
                indicator: Some(Color::rgb(0x2c, 0x3a, 0x5a)),
                ..base
            },
            DockStyle::Floating => Look {
                fill: Some(Fill::Solid(Color::CLEAR)),
                separator: Some(WHITE.with_alpha(0.25)),
                ink: Some(WHITE),
                hi: Some(Color::CLEAR),
                flat: true,
                indicator: Some(WHITE),
                ..base
            },
            DockStyle::AquaShelf => Look {
                fill: Some(Fill::Solid(Color::CLEAR)),
                separator: Some(WHITE.with_alpha(0.3)),
                ink: Some(WHITE),
                hi: Some(Color::CLEAR),
                flat: true,
                shelf: true,
                indicator: Some(Color::rgb(0x2c, 0x3a, 0x5a)),
                ..base
            },
            DockStyle::Retro => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x5f, 0x57, 0x4f)),
                ink: Some(Color::rgb(0xff, 0xf1, 0xe8)),
                indicator: Some(Color::rgb(0xff, 0x00, 0x4d)),
                ..base
            },
            DockStyle::Classic98 => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x80, 0x80, 0x80)),
                ink: Some(Color::rgb(0x00, 0x00, 0x00)),
                indicator: Some(Color::rgb(0x00, 0x00, 0x80)),
                ..base
            },
            DockStyle::Phosphor => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgb(0x1f, 0x7a, 0x3a)),
                ink: Some(Color::rgb(0x3c, 0xff, 0x6e)),
                indicator: Some(Color::rgb(0x3c, 0xff, 0x6e)),
                ..base
            },
            DockStyle::Tactical => Look {
                skin: true,
                border: false,
                separator: Some(Color::rgba(0xd9, 0xc9, 0xa3, 0.3)),
                ink: Some(Color::rgb(0xd9, 0xc9, 0xa3)),
                indicator: Some(Color::rgb(0xff, 0x8a, 0x1f)),
                ..base
            },
        }
    }

    /// The bar's fill in this palette.
    pub fn fill(&self, palette: &Palette) -> Fill {
        self.fill.clone().unwrap_or(Fill::Solid(palette.surface))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::StyleTweaks;

    fn with_style(style: DockStyle) -> DockSettings {
        DockSettings {
            style,
            ..DockSettings::default()
        }
    }

    #[test]
    fn each_style_has_its_own_look() {
        let native = Look::of(&with_style(DockStyle::Native));
        assert!(native.border);
        assert!((native.corners - 0.65).abs() < 1e-9);
        assert!(!native.reflections);
        assert_eq!(native.fill, None);

        let shelf = Look::of(&with_style(DockStyle::AquaShelf));
        assert!(!shelf.border && shelf.reflections && shelf.shelf && shelf.flat);
        assert!(
            shelf
                .fill(&Palette::of(false, Default::default()))
                .is_clear()
        );

        let neon = Look::of(&with_style(DockStyle::Neon));
        assert!(neon.outline.is_some());
        assert_eq!(neon.glows.len(), 2);
    }

    #[test]
    fn the_users_changes_go_over_the_style() {
        let mut settings = with_style(DockStyle::Neon);
        settings.tweaks.insert(
            DockStyle::Neon,
            StyleTweaks {
                border: Some(false),
                corners: Some(10),
                reflections: Some(true),
            },
        );

        let look = Look::of(&settings);

        assert!(!look.border && look.reflections);
        assert!((look.corners - 0.1).abs() < 1e-9);
        // Without the border the gradient outline goes too.
        assert_eq!(look.outline, None);
    }
}
