use crate::collection::Collection;
use crate::DirOpt;
use anyhow::Result;
use std::path::Path;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Findphotos {
    #[structopt(flatten)]
    photos: DirOpt,

    /// Base directory to search in (relative to the image root).
    base: Vec<String>,
}

impl Findphotos {
    pub fn run(&self) -> Result<()> {
        let pd = Collection::new(
            &self.photos.photos_dir,
            crate::dbopt::create_pool()?
        );
        if !self.base.is_empty() {
            for base in &self.base {
                crawl(&pd, Path::new(base))
                    .map_err(|e| anyhow!("Failed to crawl {}: {}", base, e))?;
            }
        } else {
            crawl( &pd, Path::new(""))
                .map_err(|e| anyhow!("Failed to crawl: {}", e))?;
        }
        Ok(())
    }
}

fn crawl(
    photos: &Collection,
    only_in: &Path,
) -> Result<()> {
    photos.find_files(
        only_in,
        &|path| match photos.index_photo(path) {
            Ok(()) => println!("Saved photo {}", path.display()),
            Err(e) => println!("Failed to save photo {}: {:?}", path.display(), e),
        },
    )?;
    Ok(())
}
