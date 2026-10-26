//! Shrinking a picture (spec 0009): the photo of a profile is a few hundred
//! pixels on a side and the dock draws it in a corner of an icon, so it is
//! brought down before it goes to the page. Pure: the pixels in, the pixels
//! out.

/// The largest picture read, on a side.
pub const MAX_SOURCE_SIDE: u32 = 4096;

/// RGBA pixels of `width` × `height` shrunk, keeping the shape, until neither
/// side is more than `max_side`; by the mean of the pixels each new one
/// covers. A picture that is small enough comes back as it is. `None` for one
/// with a side of 0, one bigger than [`MAX_SOURCE_SIDE`] or pixels that do
/// not match the size.
pub fn shrink(width: u32, height: u32, rgba: &[u8], max_side: u32) -> Option<(u32, u32, Vec<u8>)> {
    if width == 0 || height == 0 || max_side == 0 {
        return None;
    }
    if width > MAX_SOURCE_SIDE || height > MAX_SOURCE_SIDE {
        return None;
    }
    if rgba.len() != width as usize * height as usize * 4 {
        return None;
    }
    if width <= max_side && height <= max_side {
        return Some((width, height, rgba.to_vec()));
    }
    let scale = f64::from(max_side) / f64::from(width.max(height));
    let new_width = ((f64::from(width) * scale).round() as u32).max(1);
    let new_height = ((f64::from(height) * scale).round() as u32).max(1);
    let mut out = Vec::with_capacity(new_width as usize * new_height as usize * 4);
    for y in 0..new_height {
        let y0 = (u64::from(y) * u64::from(height) / u64::from(new_height)) as u32;
        let y1 = ((u64::from(y + 1) * u64::from(height) / u64::from(new_height)) as u32)
            .max(y0 + 1)
            .min(height);
        for x in 0..new_width {
            let x0 = (u64::from(x) * u64::from(width) / u64::from(new_width)) as u32;
            let x1 = ((u64::from(x + 1) * u64::from(width) / u64::from(new_width)) as u32)
                .max(x0 + 1)
                .min(width);
            let mut sum = [0u64; 4];
            let mut count = 0u64;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy as usize * width as usize + sx as usize) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u64::from(rgba[at + channel]);
                    }
                    count += 1;
                }
            }
            out.extend(sum.iter().map(|total| (total / count) as u8));
        }
    }
    Some((new_width, new_height, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
        pixel
            .iter()
            .copied()
            .cycle()
            .take(width as usize * height as usize * 4)
            .collect()
    }

    #[test]
    fn a_small_picture_comes_back_as_it_is() {
        let pixels = solid(8, 4, [1, 2, 3, 4]);

        assert_eq!(shrink(8, 4, &pixels, 64), Some((8, 4, pixels)));
    }

    #[test]
    fn a_big_picture_shrinks_keeping_its_shape() {
        let pixels = solid(256, 128, [200, 100, 50, 255]);

        let (width, height, out) = shrink(256, 128, &pixels, 64).unwrap();

        assert_eq!((width, height), (64, 32));
        assert_eq!(out.len(), 64 * 32 * 4);
        assert!(out.chunks(4).all(|pixel| pixel == [200, 100, 50, 255]));
    }

    #[test]
    fn each_new_pixel_is_the_mean_of_the_ones_it_covers() {
        // Two columns, black and white, shrunk to one.
        let pixels = vec![
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];

        let (width, height, out) = shrink(2, 2, &pixels, 1).unwrap();

        assert_eq!((width, height), (1, 1));
        assert_eq!(out, vec![127, 127, 127, 255]);
    }

    #[test]
    fn a_picture_that_makes_no_sense_gives_nothing() {
        let pixels = solid(4, 4, [0; 4]);

        assert_eq!(shrink(0, 4, &pixels, 2), None);
        assert_eq!(shrink(4, 4, &pixels, 0), None);
        assert_eq!(shrink(4, 4, &pixels[..10], 2), None);
        assert_eq!(shrink(MAX_SOURCE_SIDE + 1, 1, &[], 2), None);
    }
}
