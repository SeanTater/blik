use crate::models::Photo;

pub struct PhotoLink {
    pub title: Option<String>,
    pub href: String,
    pub id: String,
    pub label: Option<String>,
}

impl PhotoLink {
    pub fn date_title(p: &Photo) -> PhotoLink {
        PhotoLink {
            title: p.date.map(|d| d.format("%F").to_string()),
            href: format!("/photo/{}/thumbnail", p.id),
            id: p.id.clone(),
            label: p.date.map(|d| d.format("%T").to_string()),
        }
    }
    pub fn no_title(p: &Photo) -> PhotoLink {
        PhotoLink {
            title: None, // p.date.map(|d| d.format("%F").to_string()),
            href: format!("/photo/{}/thumbnail", p.id),
            id: p.id.clone(),
            label: p.date.map(|d| d.format("%T").to_string()),
        }
    }
}
