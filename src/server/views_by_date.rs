use std::sync::Arc;

use super::{context::GlobalContext, LoggedIn};
use super::{
    not_found, redirect_to_img, Context, ImgRange, Link, PhotoLink, RPhotosDB,
};
use crate::models::{Photo, SizeTag};
use crate::templates::{self, RenderRucte};
use anyhow::Result;
use chrono::naive::NaiveDateTime;
use chrono::{Datelike, Local};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use log::warn;
use rocket::{State, request::FlashMessage, response::content::Html};
use serde::Deserialize;
use warp::http::response::Builder;
use warp::reply::Response;

#[get("/")]
pub fn timeline(
    _user: LoggedIn,
    globe: State<Arc<GlobalContext>>,
    db: RPhotosDB,
    flash: Option<FlashMessage>
) -> Result<Html<Vec<u8>>> {
    use crate::schema::photos::dsl::photos;
    let db = db.0;
    let photo_list = photos
        .order(sql::<Integer>("date").desc())
        .limit(100)
        .load(&db)?
        .iter()
        .map(|photo: &Photo| PhotoLink {
            title: Some(format!(
                "{:04}-{:02}-{:02}",
                photo.year, photo.month, photo.day
            )),
            href: format!("/photo/{}/details", photo.id),
            label: Some(String::new()),
            id: photo.id.clone(),
            size: photo.get_size(SizeTag::Small),
        })
        .collect::<Vec<_>>();
    let context = Context::new(globe.inner().clone());
    let mut out = std::io::Cursor::new(vec![]);
    templates::index(&mut out, &context, "All photos", &[], &photo_list, &[], flash)?;
    Ok(Html(out.into_inner()))
}

pub fn on_this_day(
    context: Context,
    flash: Option<FlashMessage>
) -> Response {
    use crate::schema::photos::dsl::{date, day, grade, month, year};
    use crate::schema::positions::dsl::{
        latitude, longitude, photo_id, positions,
    };

    let (target_month, target_day) = {
        let today = Local::now();
        (today.month() as i32, today.day() as i32)
    };
    let db = context.db();
    let pos = Photo::query(context.is_authorized())
        .inner_join(positions)
        .filter(month.eq(target_month))
        .filter(day.eq(target_day))
        .select((photo_id, latitude, longitude))
        .load(&db)
        .map_err(|e| warn!("Failed to load positions: {}", e))
        .unwrap_or_default()
        .into_iter()
        .map(|(p_id, lat, long): (String, i32, i32)| {
            ((lat, long).into(), p_id)
        })
        .collect::<Vec<_>>();

    Builder::new()
        .html(|o| {
            templates::index(
                o,
                &context,
                &format!(
                    "Photos from {} {}",
                    target_day,
                    monthname(target_month)
                ),
                &[],
                &Photo::query(context.is_authorized())
                    .select(sql::<(Integer, BigInt)>("year, count(*)"))
                    .group_by(sql::<Integer>("year"))
                    .filter(month.eq(target_month as i32))
                    .filter(day.eq(target_day as i32))
                    .order(sql::<Integer>("year").desc())
                    .load::<(i32, i64)>(&db)
                    .unwrap()
                    .iter()
                    .map(|&(group_year, count)| {
                        let photo = Photo::query(context.is_authorized())
                            .filter(year.eq(group_year as i32))
                            .filter(month.eq(target_month as i32))
                            .filter(day.eq(target_day as i32))
                            .order((grade.desc(), date.asc()))
                            .limit(1)
                            .first::<Photo>(&db)
                            .unwrap();

                        PhotoLink {
                            title: Some(format!("{}", group_year)),
                            href: format!(
                                "/{}/{}/{}",
                                group_year, target_month, target_day
                            ),
                            label: Some(format!("{} pictures", count)),
                            id: photo.id.clone(),
                            size: photo.get_size(SizeTag::Small),
                        }
                    })
                    .collect::<Vec<_>>(),
                &pos,
                flash
            )
        })
        .unwrap()
}

pub fn next_image(context: Context, param: FromParam) -> Response {
    use crate::schema::photos::dsl::{date, id};
    let db = context.db();
    if let Some(from_date) = date_of_img(&db, &param.from) {
        let q = Photo::query(context.is_authorized())
            .select(id)
            .filter(
                date.gt(from_date)
                    .or(date.eq(from_date).and(id.gt(param.from))),
            )
            .order((date, id));
        if let Ok(photo) = q.first::<String>(&db) {
            return redirect_to_img(&photo);
        }
    }
    not_found(&context)
}

pub fn prev_image(context: Context, param: FromParam) -> Response {
    use crate::schema::photos::dsl::{date, id};
    let db = context.db();
    if let Some(from_date) = date_of_img(&db, &param.from) {
        let q = Photo::query(context.is_authorized())
            .select(id)
            .filter(
                date.lt(from_date)
                    .or(date.eq(from_date).and(id.lt(param.from))),
            )
            .order((date.desc(), id.desc()));
        if let Ok(photo) = q.first::<String>(&db) {
            return redirect_to_img(&photo);
        }
    }
    not_found(&context)
}

#[derive(Deserialize)]
pub struct FromParam {
    from: String,
}

pub fn date_of_img(
    db: &SqliteConnection,
    photo_id: &str,
) -> Option<NaiveDateTime> {
    use crate::schema::photos::dsl::{date, photos};
    photos.find(photo_id).select(date).first(db).unwrap_or(None)
}

pub fn monthname(n: i32) -> &'static str {
    match n {
        1 => "january",
        2 => "february",
        3 => "march",
        4 => "april",
        5 => "may",
        6 => "june",
        7 => "july",
        8 => "august",
        9 => "september",
        10 => "october",
        11 => "november",
        12 => "december",
        _ => "non-month",
    }
}
