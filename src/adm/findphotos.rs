use crate::collection::{Collection, CollectionManager};
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
        let pd = Collection {
            basedir: self.photos.photos_dir.clone(),
        };
        let conn = crate::dbopt::connect()?;
        let manager = pd.manage(&conn);
        if !self.base.is_empty() {
            for base in &self.base {
                crawl(&pd, &manager, Path::new(base))
                    .map_err(|e| anyhow!("Failed to crawl {}: {}", base, e))?;
            }
        } else {
            crawl(&pd, &manager, Path::new(""))
                .map_err(|e| anyhow!("Failed to crawl: {}", e))?;
        }
        Ok(())
    }
}

fn crawl(collection: &Collection, manager: &CollectionManager, only_in: &Path) -> Result<()> {
    collection.find_files(only_in, &|path| match manager.index_photo(path, None, "default") {
        Ok(()) => println!("Saved photo {}", path.display()),
        Err(e) => println!("Failed to save photo {}: {:?}", path.display(), e),
    })?;
    Ok(())
}
