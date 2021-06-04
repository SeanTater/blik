use std::{collections::HashMap, io::Cursor, str::FromStr};

use image::{GenericImageView, ImageEncoder};
use itertools::Itertools;
use sha2::Digest;
use anyhow::Result;

use ac_ffmpeg::codec::video::{PixelFormat, VideoDecoder, VideoFrame, VideoFrameScaler};
use ac_ffmpeg::codec::{Decoder, VideoCodecParameters};
use ac_ffmpeg::format::demuxer::{Demuxer, DemuxerWithStreamInfo};
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::packet::Packet;
use xdg_mime::SharedMimeInfo;

use crate::models::{Media, Thumbnail};

/// A simplified abstraction of a video, for the usual case where there is one video stream
pub struct VideoHandle<'t> {
    video_bytes: &'t [u8],
    demuxer: DemuxerWithStreamInfo<Cursor<&'t[u8]>>,
    vstream_dec: VideoDecoder,
    vstream_index: usize,
    vstream_params: VideoCodecParameters,
    flushed: bool
}
impl<'t> VideoHandle<'t> {
    /// Open a video file stored in a byte slice
    pub fn open(video_bytes: &'t [u8]) -> Result<VideoHandle<'t>> {
        let io = IO::from_seekable_read_stream(Cursor::new(video_bytes));
        let demuxer  = Demuxer::builder()
            .build(io)?
            .find_stream_info(None)
            .map_err(|(_, err)| err)?;
        
        let (vstream_index, vstream_params) = demuxer
            .streams()
            .iter()
            .map(|stream| stream.codec_parameters())
            .enumerate()
            .find(|(_, params)| params.is_video_codec())
            .ok_or_else(|| anyhow!("no video stream"))?;
        
        let vstream_params = vstream_params.into_video_codec_parameters().unwrap();
        
        Ok(VideoHandle {
            video_bytes,
            demuxer,
            vstream_dec: VideoDecoder::from_codec_parameters(&vstream_params)?.build()?,
            vstream_index,
            vstream_params,
            flushed: false
        })
    }

    /// Read a packet from the demuxer
    fn next_packet(&mut self) -> Result<Option<Packet>> {
        Ok(self.demuxer.take()?)
    }

    /// Read the next video frame from the first video stream in the file
    pub fn next_frame(&mut self) -> Result<Option<VideoFrame>> {
        // TODO: This seems very redundant. There must be a less repetitive way to do this.

        // Get a frame if it's already available
        if let Some(frame) = self.vstream_dec.take()? {
            return Ok(Some(frame));
        }

        // Read some more packets from the file
        while let Some(packet) = self.next_packet()? {
            // Only the packets for our video stream
            if packet.stream_index() == self.vstream_index {
                // Decode it
                self.vstream_dec.push(packet)?;

                // That packet may not have been enough to get another frame
                // but if it was, return it.
                if let Some(frame) = self.vstream_dec.take()? {
                    return Ok(Some(frame));
                }
            }
        }

        // No packets left. Squeeze out the last few frames.
        // We may still be called again, which would try to flush again
        if !self.flushed {
            self.vstream_dec.try_flush()?;
            self.flushed = true;

            // Try to get one more frame
            if let Some(frame) = self.vstream_dec.take()? {
                return Ok(Some(frame));
            }
        }

        // That's it, we're done.
        Ok(None)
    }

    /// Read Exif data from a basic image, as a reader
    ///
    /// This could be a file or an IO cursor depending on your use case
    pub fn read_media(&self, story: &str) -> anyhow::Result<Media> {
        // Start with empty metadata
        let mut result = Media::default();
        // Fill the basics
        result.id = format!("{:x}", sha2::Sha256::digest(self.video_bytes));

        // Width and height are different; we always read the image.
        result.width = self.vstream_params.width() as i32;
        result.height = self.vstream_params.height() as i32;
        let raw_metadata = self.vstream_params.extradata().unwrap_or(&[]);
        println!("Video metadata: {}", String::from_utf8_lossy(raw_metadata));


        // use crate::myexif::*;
        // use exif::*;
        // let mut cursor = std::io::Cursor::new(video_bytes);
        // let exif_map = match Reader::new().read_from_container(&mut cursor) {
        //     Ok(ex) => ex
        //         .fields()
        //         .filter(|f| f.ifd_num == In::PRIMARY)
        //         .filter_map(|f| Some((f.tag, f.clone())))
        //         .collect(),
        //     Err(x) => {
        //         log::warn!("Couldn't read EXIF: {}", x);
        //         HashMap::new()
        //     }
        // };
        // result.date = exif_map.get(&Tag::DateTimeOriginal).and_then(|f| is_datetime(f))
        //     .or_else(|| is_datetime(exif_map.get(&Tag::DateTime)?))
        //     .or_else(|| is_datetime(exif_map.get(&Tag::DateTimeDigitized)?));
        // result.make = exif_map
        //     .get(&Tag::Make)
        //     .and_then(|f| is_string(f));
        // result.model = exif_map
        //     .get(&Tag::Model)
        //     .and_then(|f| is_string(f));
        // result.rotation = exif_map
        //     .get(&Tag::Orientation)
        //     .and_then(|f| is_u32(f))
        //     .map(|value| match value {
        //         // EXIF has a funny way of encoding rotations
        //         3 => 180,
        //         6 => 90,
        //         8 => 270,
        //         1 | 0 | _ => 0,
        //     })
        //     .unwrap_or(0) as i16;
        // result.lat = exif_map
        //     .get(&Tag::GPSLatitude)
        //     .and_then(|f| is_lat_long(f));
        // result.lon = exif_map
        //     .get(&Tag::GPSLongitude)
        //     .and_then(|f| is_lat_long(f));
        // result.caption = exif_map
        //     .get(&Tag::ImageDescription)
        //     .and_then(|f| is_string(f));
        result.story = story.into();

        let mime = xdg_mime::SharedMimeInfo::new().get_mime_type_for_data(self.video_bytes)
            .ok_or(anyhow!("Can't guess mime type for video"))?
            .0;
        ensure!(mime.type_().as_str() == "video", "This video doesn't actually seem to be a video.");
        let ext = match mime.subtype().as_str() {
            "mp4" => "mp4",
            "matroska" => "mkv",
            "avi" => "avi",
            "x-motion-jpeg" => "mjpg",
            "quicktime" => "mov",
            _ => bail!("Video type {} is not supported yet. Please file an issue and we'll consider it.", mime.essence_str())
        };
        result.path = match result.date {
            Some(d) => format!("{} {}.{}", d, result.id, ext),
            None => result.id.clone()
        };
        
        Ok(result)
    }

    /// Create a thumbnail from a compressed image already read into memory
    pub fn create_thumbnail(&mut self, media: &Media) -> Result<Thumbnail> {
        // Read from a slice in memory
        let vframe = self.next_frame()?.ok_or(anyhow!("Could not read first video frame"))?;

        let height = 256;
        let width = ((256 * vframe.width()) / vframe.height()).min(2048);
        let rgb_vframe = VideoFrameScaler::builder()
            .source_pixel_format(vframe.pixel_format())
            .target_pixel_format(PixelFormat::from_str("rgb24")?)
            .source_height(vframe.height())
            .target_height(height)
            .source_width(vframe.width())
            .target_width(width)
            .build()?
            .scale(&vframe)?;
        
        // At this point we encode with `image` rather than `ffmpeg` because ffmpeg's jpg was designed
        // more for speed than quality, and more for video than photos. We also probably want to
        // support stuff like AVIF in the future, and support for that in ffmpeg is doubtful,
        // but `image` already supports it, because rav1e is in Rust
        let ref plane = rgb_vframe.planes()[0];
        let line_size = plane.line_size();
        let plane_buf = plane.data();
        ensure!(line_size >= width * 3, "Plane is too narrow, should be >= {}, but is {}", width*3, line_size);
        ensure!(plane.line_count() >= height, "Plane is too short, should be >= {}, but is {}", height, plane.line_count());

        let mut pixel_buffer = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let plane_loc = (y * line_size) + 3*x;
                let thumb_loc = (y * width + x) * 4;
                pixel_buffer[thumb_loc + 0] = plane_buf[plane_loc + 0];
                pixel_buffer[thumb_loc + 1] = plane_buf[plane_loc + 1];
                pixel_buffer[thumb_loc + 2] = plane_buf[plane_loc + 2];
                pixel_buffer[thumb_loc + 3] = 255;
            }
        }
        let mut image_serialized = std::io::Cursor::new(vec![]);

        // One day, when AVIF is in more browsers, we can do this
        // (just be sure to enable avif in the image crate)
        //
        // let image_encoder = image::codecs::avif::AvifEncoder
        //     ::new_with_speed_quality(&mut image_serialized, 5, 75);
        
        let image_encoder = image::codecs::jpeg::JpegEncoder
            ::new_with_quality(&mut image_serialized, 70);
        image_encoder.write_image(
            &mut pixel_buffer,
            width as u32,
            height as u32,
            image::ColorType::Rgba8
        )?;
        
        Ok(Thumbnail{
            id: media.id.clone(),
            content: image_serialized.into_inner(),
            mimetype: "image/jpeg".into()
        })
    }
}

