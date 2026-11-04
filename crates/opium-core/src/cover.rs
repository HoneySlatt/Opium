//! A picture chosen as a playlist's cover, made as the services take one:
//! its middle cut square, 600 px at most, as a JPEG. The phone makes its
//! own the same way with Android's decoder, which reads HEIC as well.

use image::imageops::FilterType;

/// The side of a cover made: as large as the pages show one.
pub const SIDE: u32 = 600;

/// `bytes` (a JPEG or PNG) made a cover; none if it isn't a picture.
pub fn square(bytes: &[u8]) -> Option<Vec<u8>> {
    let picture = image::load_from_memory(bytes).ok()?;
    let side = picture.width().min(picture.height());
    let cut = picture.crop_imm(
        (picture.width() - side) / 2,
        (picture.height() - side) / 2,
        side,
        side,
    );
    let sized = if side > SIDE {
        cut.resize_exact(SIDE, SIDE, FilterType::Lanczos3)
    } else {
        cut
    };
    let mut out = std::io::Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88)
        .encode_image(&sized.to_rgb8())
        .ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_picture_is_cut_square_and_brought_down() {
        let wide = image::RgbImage::from_pixel(1600, 900, image::Rgb([200, 30, 60]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(wide)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let cover = square(&png.into_inner()).unwrap();
        let made = image::load_from_memory(&cover).unwrap();
        assert_eq!((made.width(), made.height()), (SIDE, SIDE));
        assert!(square(b"not a picture").is_none());
    }
}
