//! The dock's colors (DESIGN.md "Colors"; the tokens of
//! `src/themes/tokens.css` the page's dock read): the light and dark
//! palettes, the opacity of the bar and each tile's glaze.

use crate::domain::settings::DockOpacity;

/// A color with straight (not premultiplied) alpha, 0–1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::rgba(r, g, b, 1.0)
    }

    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const CLEAR: Color = Color::rgba(0, 0, 0, 0.0);

    pub fn with_alpha(self, a: f32) -> Color {
        Color { a, ..self }
    }

    /// `hsl(h s% l%)`, with `s` and `l` in 0–100.
    pub fn hsl(h: f32, s: f32, l: f32) -> Color {
        let s = (s / 100.0).clamp(0.0, 1.0);
        let l = (l / 100.0).clamp(0.0, 1.0);
        let k = |n: f32| (n + h / 30.0).rem_euclid(12.0);
        let a = s * l.min(1.0 - l);
        let f = |n: f32| l - a * (k(n) - 3.0).min(9.0 - k(n)).clamp(-1.0, 1.0);
        Color {
            r: f(0.0),
            g: f(8.0),
            b: f(4.0),
            a: 1.0,
        }
    }

    /// `#RRGGBB`; `None` for anything else.
    pub fn hex(text: &str) -> Option<Color> {
        let digits = text.strip_prefix('#')?;
        if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
        Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?))
    }
}

/// The colors the dock is painted with in one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    pub accent: Color,
    /// The bar, with the opacity of the settings.
    pub surface: Color,
    pub ink: Color,
    pub hair: Color,
    /// The light edge on the bar.
    pub hi: Color,
    pub tip_bg: Color,
    pub tip_ink: Color,
    pub danger: Color,
}

impl Palette {
    pub fn of(dark: bool, opacity: DockOpacity) -> Palette {
        let alpha = match opacity {
            DockOpacity::Translucent => 0.8,
            DockOpacity::Solid => 1.0,
        };
        if dark {
            Palette {
                dark,
                accent: Color::rgb(0x7c, 0x9c, 0xff),
                surface: Color::rgba(38, 39, 44, alpha),
                ink: Color::rgb(0xec, 0xec, 0xf0),
                hair: Color::rgba(255, 255, 255, 0.1),
                hi: Color::rgba(255, 255, 255, 0.16),
                tip_bg: Color::rgba(50, 51, 57, 0.97),
                tip_ink: Color::rgb(0xf2, 0xf2, 0xf5),
                danger: Color::hsl(10.0, 86.0, 72.0),
            }
        } else {
            Palette {
                dark,
                accent: Color::rgb(0x2f, 0x5b, 0xd3),
                surface: Color::rgba(247, 247, 249, alpha),
                ink: Color::rgb(0x1d, 0x1e, 0x22),
                hair: Color::rgba(0, 0, 0, 0.09),
                hi: Color::rgba(255, 255, 255, 0.8),
                tip_bg: Color::rgba(252, 252, 253, 0.97),
                tip_ink: Color::rgb(0x1d, 0x1e, 0x22),
                danger: Color::hsl(10.0, 62.0, 40.0),
            }
        }
    }
}

/// The glaze of a tile: its hue, saturation and lightness (tokens.css, The
/// Own Glaze Rule).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glaze {
    pub h: f32,
    pub s: f32,
    pub l: f32,
}

impl Glaze {
    /// Petróleo.
    pub const CLOCK: Glaze = Glaze {
        h: 192.0,
        s: 26.0,
        l: 40.0,
    };
    /// Musgo.
    pub const PROGRESS: Glaze = Glaze {
        h: 84.0,
        s: 22.0,
        l: 38.0,
    };
    /// Ocre.
    pub const NOTES: Glaze = Glaze {
        h: 38.0,
        s: 48.0,
        l: 40.0,
    };
    /// Sálvia.
    pub const FILES: Glaze = Glaze {
        h: 150.0,
        s: 22.0,
        l: 44.0,
    };
    /// Vinho.
    pub const ACTIVITY: Glaze = Glaze {
        h: 350.0,
        s: 28.0,
        l: 38.0,
    };
    /// Índigo.
    pub const NETWORK: Glaze = Glaze {
        h: 248.0,
        s: 24.0,
        l: 40.0,
    };
    /// Argila: the Calendário.
    pub const CALENDAR: Glaze = Glaze {
        h: 16.0,
        s: 40.0,
        l: 44.0,
    };
    /// Ardósia rosada: the Ciclo, discreet.
    pub const CYCLE: Glaze = Glaze {
        h: 335.0,
        s: 18.0,
        l: 40.0,
    };
    /// Violeta: the Contagem regressiva.
    pub const COUNTDOWN: Glaze = Glaze {
        h: 272.0,
        s: 32.0,
        l: 44.0,
    };
    /// Tomate: the Pomodoro.
    pub const POMODORO: Glaze = Glaze {
        h: 4.0,
        s: 58.0,
        l: 47.0,
    };
    /// Folha: the Lista de tarefas.
    pub const TODO: Glaze = Glaze {
        h: 122.0,
        s: 28.0,
        l: 38.0,
    };
    /// Água: the Beber água.
    pub const WATER: Glaze = Glaze {
        h: 204.0,
        s: 58.0,
        l: 44.0,
    };
    /// Tinta.
    pub const SETTINGS: Glaze = Glaze {
        h: 222.0,
        s: 14.0,
        l: 34.0,
    };
    /// The pale plate under a program's icon.
    pub const APP: Glaze = Glaze {
        h: 220.0,
        s: 10.0,
        l: 95.0,
    };

    /// The two tones of the glaze, top and bottom.
    pub fn tones(self) -> (Color, Color) {
        (
            Color::hsl(self.h, self.s, self.l + 7.0),
            Color::hsl(self.h, self.s, self.l - 5.0),
        )
    }
}

/// The ink of the first letter that stands for a missing program icon.
pub const APP_INK: Color = Color::rgb(0x2b, 0x31, 0x40);

/// Whether Windows shows apps dark, read from the registry; light when it
/// can't be told.
#[cfg(windows)]
pub fn system_dark() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
    };
    read.is_ok() && value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(color: Color) -> (u8, u8, u8) {
        let byte = |v: f32| (v * 255.0).round() as u8;
        (byte(color.r), byte(color.g), byte(color.b))
    }

    #[test]
    fn hsl_converts_as_css_does() {
        assert_eq!(bytes(Color::hsl(0.0, 100.0, 50.0)), (255, 0, 0));
        assert_eq!(bytes(Color::hsl(120.0, 100.0, 25.0)), (0, 128, 0));
        assert_eq!(bytes(Color::hsl(10.0, 62.0, 40.0)), (165, 60, 39));
        assert_eq!(bytes(Color::hsl(220.0, 10.0, 100.0)), (255, 255, 255));
    }

    #[test]
    fn hex_reads_only_six_digits() {
        assert_eq!(Color::hex("#2f5bd3"), Some(Color::rgb(0x2f, 0x5b, 0xd3)));
        assert_eq!(Color::hex("2f5bd3"), None);
        assert_eq!(Color::hex("#fff"), None);
        assert_eq!(Color::hex("#gggggg"), None);
    }

    #[test]
    fn the_bar_follows_the_opacity_and_the_theme() {
        let light = Palette::of(false, DockOpacity::Translucent);
        let dark = Palette::of(true, DockOpacity::Solid);

        assert_eq!(light.surface, Color::rgba(247, 247, 249, 0.8));
        assert_eq!(dark.surface, Color::rgba(38, 39, 44, 1.0));
        assert_eq!(light.accent, Color::rgb(0x2f, 0x5b, 0xd3));
        assert_eq!(dark.accent, Color::rgb(0x7c, 0x9c, 0xff));
    }

    /// The value of `name` in the rule for `class` in tokens.css.
    fn token(css: &str, class: &str, name: &str) -> f32 {
        let rule = css
            .split_once(&format!(".{class} {{"))
            .and_then(|(_, rest)| rest.split_once('}'))
            .unwrap_or_else(|| panic!("no .{class} in tokens.css"))
            .0;
        let line = rule
            .lines()
            .find_map(|line| line.trim().strip_prefix(&format!("{name}:")))
            .unwrap_or_else(|| panic!("no {name} in .{class}"));
        line.trim()
            .trim_end_matches(';')
            .trim_end_matches('%')
            .parse()
            .unwrap()
    }

    #[test]
    fn the_glazes_are_the_ones_of_tokens_css() {
        let css = include_str!("../../../../src/themes/tokens.css");
        let glazes = [
            ("t-clock", Glaze::CLOCK),
            ("t-progress", Glaze::PROGRESS),
            ("t-notes", Glaze::NOTES),
            ("t-files", Glaze::FILES),
            ("t-settings", Glaze::SETTINGS),
            ("t-activity", Glaze::ACTIVITY),
            ("t-network", Glaze::NETWORK),
            ("t-calendar", Glaze::CALENDAR),
            ("t-cycle", Glaze::CYCLE),
            ("t-countdown", Glaze::COUNTDOWN),
            ("t-pomodoro", Glaze::POMODORO),
            ("t-todo", Glaze::TODO),
            ("t-water", Glaze::WATER),
        ];
        for (class, glaze) in glazes {
            let css_glaze = Glaze {
                h: token(css, class, "--h"),
                s: token(css, class, "--sat"),
                l: token(css, class, "--l"),
            };
            assert_eq!(glaze, css_glaze, "{class}");
        }
    }

    #[test]
    fn a_glaze_is_lighter_on_top() {
        let (top, bottom) = Glaze::CLOCK.tones();
        assert!(top.r + top.g + top.b > bottom.r + bottom.g + bottom.b);
    }
}
