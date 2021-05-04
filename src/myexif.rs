//! Extract all the exif data I care about
use anyhow::Result;
use chrono::{Date, Local, NaiveDate, NaiveDateTime, Utc};
use exif::{Field, In, Reader, Tag, Value};
use image::GenericImageView;
use log::{debug, error, warn};
use std::str::from_utf8;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ExifData {
    dateval: Option<NaiveDateTime>,
    gpsdate: Option<Date<Utc>>,
    gpstime: Option<(u8, u8, u8)>,
    make: Option<String>,
    model: Option<String>,
    pub width: u32,
    pub height: u32,
    orientation: Option<u32>,
    latval: Option<f64>,
    longval: Option<f64>,
    latref: Option<String>,
    longref: Option<String>,
}

impl ExifData {
    /// Read Exif data from a basic image, as a reader
    ///
    /// This could be a file or an IO cursor depending on your use case
    pub fn read_from(image_slice: &[u8]) -> Result<Self> {
        // Empty Exif struct to start
        let mut result = Self::default();
        // Width and height are different; we always read the image.
        {
            let image = image::load_from_memory(&image_slice)?;
            result.width = image.width();
            result.height = image.height();
        }
        let mut cursor = std::io::Cursor::new(image_slice);
        let reader = Reader::new().read_from_container(&mut cursor)?;
        let exif_map: HashMap<Tag, &Field> = reader
            .fields()
            .filter(|f| f.ifd_num == In::PRIMARY)
            .filter_map(|f| Some((f.tag, f)))
            .collect();
        result.dateval = exif_map
            .get(&Tag::DateTimeOriginal)
            .or(exif_map.get(&Tag::DateTime))
            .or(exif_map.get(&Tag::DateTimeDigitized))
            .and_then(|f| is_datetime(f));
        result.make = exif_map
            .get(&Tag::Make)
            .and_then(|f| is_string(f));
        result.model = exif_map
            .get(&Tag::Model)
            .and_then(|f| is_string(f));
        result.orientation = exif_map
            .get(&Tag::Orientation)
            .and_then(|f| is_u32(f));
        result.latval = exif_map
            .get(&Tag::GPSLatitude)
            .and_then(|f| is_lat_long(f));
        result.longval = exif_map
            .get(&Tag::GPSLongitude)
            .and_then(|f| is_lat_long(f));
        result.latref = exif_map
            .get(&Tag::GPSLatitudeRef)
            .and_then(|f| is_string(f));
        result.longref = exif_map
            .get(&Tag::GPSLongitudeRef)
            .and_then(|f| is_string(f));
        result.gpsdate = exif_map
            .get(&Tag::GPSDateStamp)
            .and_then(|f| is_date(f));
        result.gpstime = exif_map
            .get(&Tag::GPSTimeStamp)
            .and_then(|f| is_time(f));
        
        Ok(result)
    }

    pub fn date(&self) -> Option<NaiveDateTime> {
        // Note: I probably return and store datetime with tz,
        // possibly utc, instead.
        if let (&Some(date), &Some((h, m, s))) = (&self.gpsdate, &self.gpstime)
        {
            let naive = date
                .and_hms(u32::from(h), u32::from(m), u32::from(s))
                .with_timezone(&Local)
                .naive_local();
            debug!("GPS Date {}, {}:{}:{} => {}", date, h, m, s, naive);
            Some(naive)
        } else if let Some(date) = self.dateval {
            Some(date)
        } else {
            warn!("No date found in exif");
            None
        }
        .filter(|d| d != &NaiveDateTime::from_timestamp(0, 0))
        .filter(|d| d != &NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0))
    }
    pub fn position(&self) -> Option<(f64, f64)> {
        if let (Some(lat), Some(long)) = (self.lat(), self.long()) {
            Some((lat, long))
        } else {
            None
        }
    }
    fn lat(&self) -> Option<f64> {
        match (&self.latref, self.latval) {
            (&Some(ref r), Some(lat)) if r == "N" => Some(lat.abs()),
            (&Some(ref r), Some(lat)) if r == "S" => Some(-(lat.abs())),
            (&Some(ref r), lat) => {
                error!("Bad latref: {}", r);
                lat
            }
            (&None, lat) => lat,
        }
    }
    fn long(&self) -> Option<f64> {
        match (&self.longref, self.longval) {
            (&Some(ref r), Some(long)) if r == "E" => Some(long.abs()),
            (&Some(ref r), Some(long)) if r == "W" => Some(-(long.abs())),
            (&Some(ref r), long) => {
                error!("Bad longref: {}", r);
                long
            }
            (&None, long) => long,
        }
    }

    pub fn rotation(&self) -> Result<i16> {
        if let Some(value) = self.orientation {
            debug!("Raw orientation is {}", value);
            match value {
                1 | 0 => Ok(0),
                3 => Ok(180),
                6 => Ok(90),
                8 => Ok(270),
                x => Err(anyhow!("Unknown orientation: {}", x)),
            }
        } else {
            debug!("Orientation tag missing, default to 0 degrees");
            Ok(0)
        }
    }
}

fn is_lat_long(f: &Field) -> Option<f64> {
    match f.value {
        Value::Rational(ref v) if v.len() == 3 => {
            let d = 1. / 60.;
            Some(v[0].to_f64() + d * (v[1].to_f64() + d * v[2].to_f64()))
        }
        ref v => {
            println!("ERROR: Bad value for {}: {:?}", f.tag, v);
            None
        }
    }
}

fn is_datetime(f: &Field) -> Option<NaiveDateTime> {
    single_ascii(&f.value)
        .and_then(|s| Ok(NaiveDateTime::parse_from_str(s, "%Y:%m:%d %T")?))
        .map_err(|e| {
            println!("ERROR: Expected datetime for {} (which was {:?}): {:?}", f.tag, &f.value, e);
        })
        .ok()
}

fn is_date(f: &Field) -> Option<Date<Utc>> {
    single_ascii(&f.value)
        .and_then(|s| Ok(NaiveDate::parse_from_str(s, "%Y:%m:%d")?))
        .map(|d| Date::from_utc(d, Utc))
        .map_err(|e| {
            println!("ERROR: Expected date for {}: {:?}", f.tag, e);
        })
        .ok()
}

fn is_time(f: &Field) -> Option<(u8, u8, u8)> {
    match &f.value {
        // Some cameras (notably iPhone) uses fractional seconds.
        // Just round to whole seconds.
        &Value::Rational(ref v)
            if v.len() == 3 && v[0].denom == 1 && v[1].denom == 1 =>
        {
            Some((
                v[0].num as u8,
                v[1].num as u8,
                v[2].to_f64().round() as u8,
            ))
        }
        err => {
            error!("Expected time for {}: {:?}", f.tag, err);
            None
        }
    }
}

fn is_string(f: &Field) -> Option<String> {
    match single_ascii(&f.value) {
        Ok(s) => Some(s.to_string()),
        Err(err) => {
            println!("ERROR: Expected string for {}: {:?}", f.tag, err);
            None
        }
    }
}

fn is_u32(f: &Field) -> Option<u32> {
    match &f.value {
        &Value::Long(ref v) if v.len() == 1 => Some(v[0]),
        &Value::Short(ref v) if v.len() == 1 => Some(u32::from(v[0])),
        v => {
            println!("ERROR: Unsuppored value for {}: {:?}", f.tag, v);
            None
        }
    }
}

fn single_ascii(value: &Value) -> Result<&str> {
    match value {
        &Value::Ascii(ref v) if v.len() == 1 => Ok(from_utf8(&v[0])?),
        &Value::Ascii(ref v) if v.len() > 1 => {
            for t in &v[1..] {
                if !t.is_empty() {
                    return Err(anyhow!(
                        "Got {:?}, expected single ascii value",
                        v,
                    ));
                }
            }
            Ok(from_utf8(&v[0])?)
        }
        v => Err(anyhow!("Got {:?}, expected single ascii value", v,)),
    }
}
