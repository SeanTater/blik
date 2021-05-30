use std::collections::HashMap;

use image::GenericImageView;
use sha2::Digest;
use anyhow::Result;

use crate::models::{Media, Thumbnail};

/// Read Exif data from a basic image, as a reader
///
/// This could be a file or an IO cursor depending on your use case
pub fn read_media_from(image_slice: &[u8], story: &str) -> anyhow::Result<Media> {
    use crate::myexif::*;
    use exif::*;

    // Start with an empty photo
    let mut result = Media::default();
    // Fill the basics
    result.id = format!("{:x}", sha2::Sha256::digest(&image_slice));

    // Width and height are different; we always read the image.
    {
        let image = image::load_from_memory(&image_slice)?;
        result.width = image.width() as i32;
        result.height = image.height() as i32;
    }
    let mut cursor = std::io::Cursor::new(image_slice);
    let exif_map = match Reader::new().read_from_container(&mut cursor) {
        Ok(ex) => ex
            .fields()
            .filter(|f| f.ifd_num == In::PRIMARY)
            .filter_map(|f| Some((f.tag, f.clone())))
            .collect(),
        Err(x) => {
            log::warn!("Couldn't read EXIF: {}", x);
            HashMap::new()
        }
    };
    result.date = exif_map.get(&Tag::DateTimeOriginal).and_then(|f| is_datetime(f))
        .or_else(|| is_datetime(exif_map.get(&Tag::DateTime)?))
        .or_else(|| is_datetime(exif_map.get(&Tag::DateTimeDigitized)?));
    result.make = exif_map
        .get(&Tag::Make)
        .and_then(|f| is_string(f));
    result.model = exif_map
        .get(&Tag::Model)
        .and_then(|f| is_string(f));
    result.rotation = exif_map
        .get(&Tag::Orientation)
        .and_then(|f| is_u32(f))
        .map(|value| match value {
            // EXIF has a funny way of encoding rotations
            3 => 180,
            6 => 90,
            8 => 270,
            1 | 0 | _ => 0,
        })
        .unwrap_or(0) as i16;
    result.lat = exif_map
        .get(&Tag::GPSLatitude)
        .and_then(|f| is_lat_long(f));
    result.lon = exif_map
        .get(&Tag::GPSLongitude)
        .and_then(|f| is_lat_long(f));
    result.caption = exif_map
        .get(&Tag::ImageDescription)
        .and_then(|f| is_string(f));
    result.story = story.into();
    

    let ext = *image::guess_format(image_slice)?
        .extensions_str()
        .first()
        .unwrap_or(&"image");
    result.path = match result.date {
        Some(d) => format!("{} {}.{}", d, result.id, ext),
        None => result.id.clone()
    };
    
    Ok(result)
}

/// Create a thumbnail from a compressed image already read into memory
pub fn create_thumbnail(media: &Media, image_bytes: &[u8]) -> Result<Thumbnail> {
    let mut thumbnail_buf = std::io::Cursor::new(vec![]);
    let thumbnail = image::load_from_memory(&image_bytes)?
        .thumbnail(256, 256);
    // Rotate if necessary before saving
    let thumbnail = match media.rotation {
        90 | -270 => thumbnail.rotate90(),
        180 | -180  => thumbnail.rotate180(),
        270 | -90 => thumbnail.rotate270(),
        _ => thumbnail
    };
    thumbnail.write_to(&mut thumbnail_buf, image::ImageOutputFormat::Jpeg(80))?;
    let content = thumbnail_buf.into_inner();
    Ok(Thumbnail{id: media.id.clone(), content})
}