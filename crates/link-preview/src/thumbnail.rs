//! Thumbnails and site icons: a downloaded image made small enough to keep on the phone.

use std::io::Cursor;

use image::{ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};

use crate::{Error, Result};

/// Longest side of a thumbnail, in pixels: sharp across a phone's width on a high-density screen
/// (the detail screen), so the cards' smaller version is sharp too.
pub const THUMBNAIL_SIZE: u32 = 800;

/// JPEG quality: small files, no visible loss at this size.
const QUALITY: u8 = 80;
/// Larger images are refused rather than decoded.
const MAX_DECODE_SIDE: u32 = 8000;
const MAX_DECODE_BYTES: u64 = 160 * 1024 * 1024;

/// Longest side of a stored site icon, in pixels.
pub const ICON_SIZE: u32 = 128;

/// Decodes a JPEG, PNG, WebP or GIF (first frame), shrinks it to at most `max_side` pixels on
/// its longest side (never enlarges), and encodes it as JPEG. Transparency becomes white.
pub fn thumbnail(bytes: &[u8], max_side: u32) -> Result<Vec<u8>> {
  let image = decode(bytes, max_side)?;

  // JPEG has no alpha: paint transparent pixels onto white.
  let mut rgb = image::RgbImage::new(image.width(), image.height());
  for (x, y, pixel) in image.to_rgba8().enumerate_pixels() {
    let [r, g, b, a] = pixel.0;
    let over_white = |c: u8| ((c as u16 * a as u16 + 255 * (255 - a as u16)) / 255) as u8;
    rgb.put_pixel(x, y, image::Rgb([over_white(r), over_white(g), over_white(b)]));
  }

  let mut out = Vec::new();
  JpegEncoder::new_with_quality(&mut out, QUALITY)
    .encode_image(&rgb)
    .map_err(|e| Error::BadImage(e.to_string()))?;
  Ok(out)
}

/// A site icon (`.ico`, PNG, …) for the phone: at most `max_side` pixels, as PNG so a
/// transparent background stays transparent. From a multi-size `.ico`, the largest image.
pub fn icon(bytes: &[u8], max_side: u32) -> Result<Vec<u8>> {
  let image = decode(bytes, max_side)?;
  let mut out = Vec::new();
  image
    .to_rgba8()
    .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
    .map_err(|e| Error::BadImage(e.to_string()))?;
  Ok(out)
}

/// Decodes any supported format within the size limits and shrinks it to `max_side`.
fn decode(bytes: &[u8], max_side: u32) -> Result<image::DynamicImage> {
  let mut reader = ImageReader::new(Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|e| Error::BadImage(e.to_string()))?;
  // A small file can claim a huge size; refuse before allocating for it.
  let mut limits = image::Limits::default();
  limits.max_image_width = Some(MAX_DECODE_SIDE);
  limits.max_image_height = Some(MAX_DECODE_SIDE);
  limits.max_alloc = Some(MAX_DECODE_BYTES);
  reader.limits(limits);
  let image = reader
    .decode()
    .map_err(|e| Error::BadImage(e.to_string()))?;
  Ok(if image.width().max(image.height()) > max_side {
    image.resize(max_side, max_side, FilterType::Triangle)
  } else {
    image
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn png(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
    let mut out = Vec::new();
    image.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
    out
  }

  fn decoded(jpeg: &[u8]) -> image::DynamicImage {
    image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).unwrap()
  }

  #[test]
  fn shrinks_the_longest_side_and_keeps_the_shape() {
    let thumb = decoded(&thumbnail(&png(1200, 600, [10, 120, 200, 255]), 400).unwrap());
    assert_eq!((thumb.width(), thumb.height()), (400, 200));
  }

  #[test]
  fn never_enlarges() {
    let thumb = decoded(&thumbnail(&png(64, 32, [0, 0, 0, 255]), 400).unwrap());
    assert_eq!((thumb.width(), thumb.height()), (64, 32));
  }

  #[test]
  fn transparency_becomes_white() {
    let thumb = decoded(&thumbnail(&png(8, 8, [0, 0, 0, 0]), 400).unwrap()).to_rgb8();
    assert!(thumb.pixels().all(|p| p.0.iter().all(|&c| c > 245)));
  }

  #[test]
  fn icons_keep_transparency_and_come_out_as_png() {
    let png_icon = icon(&png(256, 256, [200, 30, 30, 0]), 128).unwrap();
    let decoded = image::load_from_memory_with_format(&png_icon, image::ImageFormat::Png).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (128, 128));
    assert_eq!(decoded.to_rgba8().get_pixel(0, 0).0[3], 0);
  }

  #[test]
  fn reads_ico_files() {
    // A 16×16 and a 48×48 image in one .ico: the larger one is used.
    let mut ico = Vec::new();
    let frames: Vec<_> = [16u32, 48]
      .iter()
      .map(|&size| {
        let img = image::RgbaImage::from_pixel(size, size, image::Rgba([0, 90, 200, 255]));
        image::codecs::ico::IcoFrame::as_png(img.as_raw(), size, size, image::ExtendedColorType::Rgba8).unwrap()
      })
      .collect();
    image::codecs::ico::IcoEncoder::new(&mut ico).encode_images(&frames).unwrap();
    let out = image::load_from_memory_with_format(&icon(&ico, 128).unwrap(), image::ImageFormat::Png).unwrap();
    assert_eq!((out.width(), out.height()), (48, 48));
  }

  #[test]
  fn huge_images_are_refused() {
    assert!(matches!(thumbnail(&png(9000, 10, [0, 0, 0, 255]), 400), Err(Error::BadImage(_))));
  }

  #[test]
  fn other_bytes_are_an_error() {
    assert!(matches!(thumbnail(b"<html>not an image</html>", 400), Err(Error::BadImage(_))));
  }
}
