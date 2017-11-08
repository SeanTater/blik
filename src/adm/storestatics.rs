use adm::result::Error;
use brotli2::write::BrotliEncoder;
use flate2::{Compression, FlateWriteExt};
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::path::Path;
use templates::statics::STATICS;

pub fn to_dir(dir: &str) -> Result<(), Error> {
    let dir: &Path = dir.as_ref();
    create_dir_all(&dir)?;
    for s in STATICS {
        File::create(dir.join(s.name)).and_then(|mut f| f.write(s.content))?;

        File::create(dir.join(format!("{}.gz", s.name)))
            .map(|f| f.gz_encode(Compression::Best))
            .and_then(|mut f| f.write(s.content))?;

        File::create(dir.join(format!("{}.br", s.name)))
            .map(|f| BrotliEncoder::new(f, 11))
            .and_then(|mut f| f.write(s.content))?;
    }
    Ok(())
}
