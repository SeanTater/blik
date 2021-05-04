//! Extract all the exif data I care about
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use exif::{Field, Value};
use std::str::from_utf8;

pub fn is_lat_long(f: &Field) -> Option<f64> {
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

pub fn is_datetime(f: &Field) -> Option<NaiveDateTime> {
    single_ascii(&f.value)
        .and_then(|s| Ok(NaiveDateTime::parse_from_str(s, "%Y:%m:%d %T")?))
        .map_err(|e| {
            println!("ERROR: Expected datetime for {} (which was {:?}): {:?}", f.tag, &f.value, e);
        })
        .ok()
        .filter(|d| 
            d != &NaiveDateTime::from_timestamp(0, 0)
            && d != &NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)
        )
}

pub fn is_string(f: &Field) -> Option<String> {
    match single_ascii(&f.value) {
        Ok(s) => Some(s.to_string()),
        Err(err) => {
            println!("ERROR: Expected string for {}: {:?}", f.tag, err);
            None
        }
    }
}

pub fn is_u32(f: &Field) -> Option<u32> {
    match &f.value {
        &Value::Long(ref v) if v.len() == 1 => Some(v[0]),
        &Value::Short(ref v) if v.len() == 1 => Some(u32::from(v[0])),
        v => {
            println!("ERROR: Unsuppored value for {}: {:?}", f.tag, v);
            None
        }
    }
}

pub fn single_ascii(value: &Value) -> Result<&str> {
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
