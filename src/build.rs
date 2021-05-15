use ructe::{Ructe, RucteError};

fn main() -> Result<(), RucteError> {
    let mut ructe = Ructe::from_env()?;
    let mut statics = ructe.statics()?;
    statics.add_sass_file("static/sass/photos.scss")?;
    statics.add_files("static/raw")?;
    ructe.compile_templates("templates")?;
    Ok(())
}
