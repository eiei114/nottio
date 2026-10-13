//! Clipboard bitmaps become self-contained PNGs in the board document. No temp
//! filename or network URL is needed to reopen, export or copy an image object.

use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use gpui::{ClipboardEntry, ClipboardItem, Image, ImageFormat, ImageSource};
use image::{DynamicImage, ImageDecoder, ImageReader};

const PREFIX: &str = "data:image/png;base64,";
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_PIXELS: u64 = 16_000_000;
const CACHE_BYTES: usize = 128 * 1024 * 1024;

fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    if bytes.len() > MAX_BYTES {
        return Err("画像は32MB以下にしてください".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err("画像は1600万画素以下にしてください".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut bitmap = DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    bitmap.apply_orientation(orientation);
    Ok(bitmap)
}

fn png(bitmap: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    bitmap
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    if out.get_ref().len() > MAX_BYTES {
        return Err("PNG変換後の画像は32MB以下にしてください".into());
    }
    Ok(out.into_inner())
}

/// Read the first bitmap, including the BMP produced from Windows CF_DIB.
/// File paths and plain text are intentionally not interpreted as images.
pub(crate) fn from_clipboard(item: &ClipboardItem) -> Result<Option<(String, f32, f32)>, String> {
    let Some(bytes) = item.entries().iter().find_map(|entry| match entry {
        ClipboardEntry::Image(image) => Some(image.bytes.as_slice()),
        _ => None,
    }) else {
        return Ok(None);
    };
    let bitmap = decode(bytes)?;
    let (w, h) = (bitmap.width() as f32, bitmap.height() as f32);
    Ok(Some((
        format!("{PREFIX}{}", STANDARD.encode(png(&bitmap)?)),
        w,
        h,
    )))
}

/// Host-side LRU of encoded quarter-turn variants. GPUI decodes/uploads the
/// returned image asynchronously. Sources outside our embedded PNG format are
/// not loaded: opening a board never starts arbitrary HTTP or local-file reads.
#[derive(Default)]
pub(crate) struct ImageCache {
    entries: VecDeque<((u64, u8), Arc<Image>)>,
    bytes: usize,
}

impl ImageCache {
    pub(crate) fn source(&mut self, src: &str, rotation: f32) -> Option<ImageSource> {
        if !rotation.is_finite() || src.len() > PREFIX.len() + MAX_BYTES.div_ceil(3) * 4 {
            return None;
        }
        let encoded = src.strip_prefix(PREFIX)?;
        let turn = (rotation / std::f32::consts::FRAC_PI_2).round() as i32;
        let turn = turn.rem_euclid(4) as u8;
        let key = (gpui::hash(&src), turn);
        if let Some(index) = self.entries.iter().position(|(k, _)| *k == key) {
            let entry = self.entries.remove(index)?;
            let source = ImageSource::Image(entry.1.clone());
            self.entries.push_back(entry);
            return Some(source);
        }
        let bytes = STANDARD.decode(encoded).ok()?;
        let bitmap = decode(&bytes).ok()?;
        let bytes = match turn {
            0 => bytes,
            1 => png(&bitmap.rotate90()).ok()?,
            2 => png(&bitmap.rotate180()).ok()?,
            _ => png(&bitmap.rotate270()).ok()?,
        };
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, bytes));
        while self.bytes + image.bytes.len() > CACHE_BYTES {
            let (_, old) = self.entries.pop_front()?;
            self.bytes -= old.bytes.len();
        }
        self.bytes += image.bytes.len();
        self.entries.push_back((key, image.clone()));
        Some(ImageSource::Image(image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clipboard(format: image::ImageFormat) -> ClipboardItem {
        let bitmap = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            6,
            3,
            image::Rgba([220, 80, 30, 128]),
        ));
        let mut bytes = Cursor::new(Vec::new());
        bitmap.write_to(&mut bytes, format).unwrap();
        ClipboardItem::new_image(&Image::from_bytes(ImageFormat::Png, bytes.into_inner()))
    }

    #[test]
    fn clipboard_png_is_portable_and_preserves_alpha_after_save_restore() {
        let (src, w, h) = from_clipboard(&clipboard(image::ImageFormat::Png))
            .unwrap()
            .unwrap();
        assert_eq!((w, h), (6.0, 3.0));
        let saved = serde_json::to_string(&src).unwrap();
        let restored: String = serde_json::from_str(&saved).unwrap();
        let mut cache = ImageCache::default();
        let ImageSource::Image(image) = cache.source(&restored, 0.0).unwrap() else {
            panic!()
        };
        let pixels = decode(&image.bytes).unwrap().into_rgba8();
        assert_eq!(pixels.get_pixel(0, 0).0, [220, 80, 30, 128]);
        let ImageSource::Image(again) = cache.source(&restored, 0.0).unwrap() else {
            panic!()
        };
        assert!(Arc::ptr_eq(&image, &again));
        let ImageSource::Image(rotated) = cache
            .source(&restored, std::f32::consts::FRAC_PI_2)
            .unwrap()
        else {
            panic!()
        };
        let rotated = decode(&rotated.bytes).unwrap();
        assert_eq!((rotated.width(), rotated.height()), (3, 6));
    }

    #[test]
    fn clipboard_bmp_and_jpeg_normalize_to_png() {
        for format in [image::ImageFormat::Bmp, image::ImageFormat::Jpeg] {
            // JPEG cannot encode alpha; use RGB for this format.
            let item = if format == image::ImageFormat::Jpeg {
                let mut bytes = Cursor::new(Vec::new());
                DynamicImage::new_rgb8(6, 3)
                    .write_to(&mut bytes, format)
                    .unwrap();
                ClipboardItem::new_image(&Image::from_bytes(ImageFormat::Jpeg, bytes.into_inner()))
            } else {
                clipboard(format)
            };
            let (src, w, h) = from_clipboard(&item).unwrap().unwrap();
            assert!(src.starts_with(PREFIX));
            assert_eq!((w, h), (6.0, 3.0));
        }
    }

    #[test]
    fn macos_tiff_clipboard_normalizes_to_portable_png() {
        let (src, w, h) = from_clipboard(&clipboard(image::ImageFormat::Tiff))
            .unwrap()
            .unwrap();
        assert_eq!((w, h), (6.0, 3.0));
        let bytes = STANDARD.decode(src.strip_prefix(PREFIX).unwrap()).unwrap();
        assert_eq!(
            image::guess_format(&bytes).unwrap(),
            image::ImageFormat::Png
        );
        assert_eq!(
            decode(&bytes).unwrap().into_rgba8().get_pixel(0, 0).0,
            [220, 80, 30, 128]
        );
    }

    #[test]
    fn invalid_unsupported_and_oversized_images_are_rejected() {
        assert!(from_clipboard(&ClipboardItem::new_string("text".into()))
            .unwrap()
            .is_none());
        assert!(from_clipboard(&ClipboardItem::new_image(&Image::empty())).is_err());
        assert!(decode(&vec![0; MAX_BYTES + 1]).is_err());
        let mut cache = ImageCache::default();
        for src in [
            "https://example.com/image.png",
            "C:/private.png",
            "data:image/png;base64,bad!",
        ] {
            assert!(cache.source(src, 0.0).is_none());
        }
    }
}
