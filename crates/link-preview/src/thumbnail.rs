//! Thumbnails: a downloaded image made small enough to keep on the phone.

use std::io::Cursor;

use image::{ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};

use crate::{Error, Result};

/// Longest side of a thumbnail, in pixels: sharp in a list on a high-density screen.
pub const THUMBNAIL_SIZE: u32 = 400;

/// JPEG quality: small files, no visible loss at this size.
const QUALITY: u8 = 80;
/// Larger images are refused rather than decoded.
const MAX_DECODE_SIDE: u32 = 8000;
const MAX_DECODE_BYTES: u64 = 160 * 1024 * 1024;

/// Decodes a JPEG, PNG, WebP or GIF (first frame), shrinks it to at most `max_side` pixels on
/// its longest side (never enlarges), and encodes it as JPEG. Transparency becomes white.
pub fn thumbnail(bytes: &[u8], max_side: u32) -> Result<Vec<u8>> {
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
  let image = if image.width().max(image.height()) > max_side {
    image.resize(max_side, max_side, FilterType::Triangle)
  } else {
    image
  };

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
  fn huge_images_are_refused() {
    assert!(matches!(thumbnail(&png(9000, 10, [0, 0, 0, 255]), 400), Err(Error::BadImage(_))));
  }

  #[test]
  fn other_bytes_are_an_error() {
    assert!(matches!(thumbnail(b"<html>not an image</html>", 400), Err(Error::BadImage(_))));
  }
}
