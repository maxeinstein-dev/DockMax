//! The skins of the bar (spec 0020 §3): a style whose bar is an image, cut
//! in nine slices. The top and bottom slices are empty: the whole image goes
//! to the bar's height, so only its width changes. The left and right ends
//! keep their drawing; the middle repeats (or stretches) to the bar's width.
//!
//! A skin is `skin.json` and `skin.png` in `src-tauri/skins/<id>/`, made by
//! `src-tauri/skins/fonte/gerar.py` from a drawing in text. They are in the
//! binary; the miniatures of Configurações read the same files.

use std::sync::OnceLock;

use serde::Deserialize;

use super::settings::DockStyle;

/// What `skin.json` says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SkinMeta {
    pub name: String,
    pub image: String,
    pub width: u32,
    pub height: u32,
    pub slice: Slice,
    pub middle: Middle,
    /// Scaled up without smoothing (pixel art).
    pub pixelated: bool,
}

/// The nine-slice cut, in pixels of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Slice {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

/// How the middle fills the bar's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Middle {
    Repeat,
    Stretch,
}

/// A skin in the binary.
pub struct Skin {
    pub id: &'static str,
    pub png: &'static [u8],
    json: &'static str,
}

macro_rules! skin {
    ($id:literal) => {
        Skin {
            id: $id,
            png: include_bytes!(concat!("../../skins/", $id, "/skin.png")),
            json: include_str!(concat!("../../skins/", $id, "/skin.json")),
        }
    };
}

/// Every skin in the binary.
pub static SKINS: [Skin; 6] = [
    skin!("classic98"),
    skin!("phosphor"),
    skin!("retro"),
    skin!("tactical"),
    skin!("medieval"),
    skin!("hunter"),
];

impl Skin {
    /// Its `skin.json`, read once.
    pub fn meta(&self) -> &'static SkinMeta {
        static METAS: OnceLock<Vec<SkinMeta>> = OnceLock::new();
        let metas = METAS.get_or_init(|| {
            SKINS
                .iter()
                .map(|skin| serde_json::from_str(skin.json).expect("a bundled skin.json is valid"))
                .collect()
        });
        let index = SKINS
            .iter()
            .position(|skin| skin.id == self.id)
            .expect("a bundled skin");
        &metas[index]
    }
}

/// The skin of `style`'s bar; `None` for a style painted with colors.
pub fn skin_of(style: DockStyle) -> Option<&'static Skin> {
    let id = match style {
        DockStyle::Classic98 => "classic98",
        DockStyle::Phosphor => "phosphor",
        DockStyle::Retro => "retro",
        DockStyle::Tactical => "tactical",
        DockStyle::Medieval => "medieval",
        DockStyle::Hunter => "hunter",
        _ => return None,
    };
    SKINS.iter().find(|skin| skin.id == id)
}

/// A rectangle: `x`, `y`, width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A part of the image (`from`, in its pixels) drawn at `to`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub from: Rect,
    pub to: Rect,
}

/// The pieces that draw `meta`'s image over `bar`: the left end, the middle
/// (in tiles when it repeats, the last one cut) and the right end, all at
/// the bar's height. A bar narrower than the two ends shrinks them.
pub fn pieces(meta: &SkinMeta, bar: Rect) -> Vec<Piece> {
    if meta.height == 0 || bar.width <= 0.0 || bar.height <= 0.0 {
        return Vec::new();
    }
    let height = f64::from(meta.height);
    let k = bar.height / height;
    let (left, right) = (f64::from(meta.slice.left), f64::from(meta.slice.right));
    let middle = f64::from(meta.width) - left - right;
    let ends = (left + right) * k;
    let squeeze = if ends > bar.width {
        bar.width / ends
    } else {
        1.0
    };
    let (left_to, right_to) = (left * k * squeeze, right * k * squeeze);
    let at = |x: f64, width: f64| Rect {
        x,
        y: bar.y,
        width,
        height: bar.height,
    };
    let from = |x: f64, width: f64| Rect {
        x,
        y: 0.0,
        width,
        height,
    };
    let mut pieces = vec![Piece {
        from: from(0.0, left),
        to: at(bar.x, left_to),
    }];
    let span = bar.width - left_to - right_to;
    if span > 0.0 && middle > 0.0 {
        match meta.middle {
            Middle::Stretch => pieces.push(Piece {
                from: from(left, middle),
                to: at(bar.x + left_to, span),
            }),
            Middle::Repeat => {
                let tile = middle * k;
                let mut x = 0.0;
                while x < span - 1e-6 {
                    let width = tile.min(span - x);
                    pieces.push(Piece {
                        from: from(left, width / k),
                        to: at(bar.x + left_to + x, width),
                    });
                    x += tile;
                }
            }
        }
    }
    pieces.push(Piece {
        from: from(f64::from(meta.width) - right, right),
        to: at(bar.x + bar.width - right_to, right_to),
    });
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(width: u32, left: u32, right: u32, middle: Middle) -> SkinMeta {
        SkinMeta {
            name: "Teste".into(),
            image: "skin.png".into(),
            width,
            height: 16,
            slice: Slice {
                top: 0,
                right,
                bottom: 0,
                left,
            },
            middle,
            pixelated: true,
        }
    }

    const BAR: Rect = Rect {
        x: 10.0,
        y: 4.0,
        width: 100.0,
        height: 64.0,
    };

    #[test]
    fn the_ends_keep_their_drawing_at_the_bar_s_height() {
        let pieces = pieces(&meta(5, 2, 2, Middle::Stretch), BAR);
        assert_eq!(pieces.len(), 3);
        // 64 / 16: every pixel of the image is 4 of the bar.
        assert_eq!(
            pieces[0].from,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 16.0
            }
        );
        assert_eq!(
            pieces[0].to,
            Rect {
                x: 10.0,
                y: 4.0,
                width: 8.0,
                height: 64.0
            }
        );
        assert_eq!(
            pieces[2].from,
            Rect {
                x: 3.0,
                y: 0.0,
                width: 2.0,
                height: 16.0
            }
        );
        assert_eq!(
            pieces[2].to,
            Rect {
                x: 102.0,
                y: 4.0,
                width: 8.0,
                height: 64.0
            }
        );
        // The middle stretches between them.
        assert_eq!(
            pieces[1].to,
            Rect {
                x: 18.0,
                y: 4.0,
                width: 84.0,
                height: 64.0
            }
        );
    }

    #[test]
    fn a_repeating_middle_comes_in_tiles_the_last_one_cut() {
        // A 3 px middle is 12 px on the bar; 84 px of middle is 7 tiles.
        let tiled = pieces(&meta(7, 2, 2, Middle::Repeat), BAR);
        let middle = &tiled[1..tiled.len() - 1];
        assert_eq!(middle.len(), 7);
        assert!(
            middle
                .iter()
                .all(|piece| (piece.to.width - 12.0).abs() < 1e-9)
        );
        let wide = Rect { width: 90.0, ..BAR };
        let cut = pieces(&meta(7, 2, 2, Middle::Repeat), wide);
        let last = cut[cut.len() - 2];
        // 74 px of middle: six whole tiles and 2 px, 0.5 px of the image.
        assert!((last.to.width - 2.0).abs() < 1e-9);
        assert!((last.from.width - 0.5).abs() < 1e-9);
        let covered: f64 = cut.iter().map(|piece| piece.to.width).sum();
        assert!((covered - 90.0).abs() < 1e-9);
    }

    #[test]
    fn a_bar_narrower_than_the_ends_shrinks_them() {
        let narrow = Rect { width: 10.0, ..BAR };
        let pieces = pieces(&meta(5, 2, 2, Middle::Repeat), narrow);
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0].to.width - 5.0).abs() < 1e-9);
        assert!((pieces[1].to.x - 15.0).abs() < 1e-9);
    }

    #[test]
    fn every_bundled_skin_is_what_its_png_and_its_styles_say() {
        for skin in &SKINS {
            let meta = skin.meta();
            let size = crate::domain::icon_pack::png_size(skin.png).expect("a PNG");
            assert_eq!(size, (meta.width, meta.height), "{}", skin.id);
            assert_eq!((meta.slice.top, meta.slice.bottom), (0, 0), "{}", skin.id);
            assert!(
                meta.slice.left + meta.slice.right < meta.width,
                "{}",
                skin.id
            );
            assert_eq!(meta.image, "skin.png", "{}", skin.id);
        }
        for (style, id) in [
            (DockStyle::Classic98, "classic98"),
            (DockStyle::Phosphor, "phosphor"),
            (DockStyle::Retro, "retro"),
            (DockStyle::Tactical, "tactical"),
            (DockStyle::Medieval, "medieval"),
            (DockStyle::Hunter, "hunter"),
        ] {
            assert_eq!(skin_of(style).map(|skin| skin.id), Some(id));
        }
        assert!(skin_of(DockStyle::Native).is_none());
    }
}
