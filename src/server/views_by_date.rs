use super::splitlist::links_by_time;
use super::{not_found, redirect_to_img, Context, ImgRange, Link, PhotoLink};
use crate::models::{Photo, SizeTag};
use crate::templates::{self, RenderRucte};
use chrono::naive::NaiveDateTime;
use chrono::{Datelike, Local};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use log::warn;
use serde::Deserialize;
use warp::http::response::Builder;
use warp::reply::Response;

pub fn all_years(context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade, year};
    let db = context.db();
    let groups = Photo::query(context.is_authorized())
        .select(sql::<(Integer, BigInt)>("year, count(*)"))
        .group_by(sql::<Integer>("year"))
        .order(sql::<Integer>("year").desc())
        .load::<(i32, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(group_year, count)| {
            let photo: Photo = Photo::query(context.is_authorized())
                .filter(year.eq(group_year))
                .order((grade.desc(), date.asc()))
                .limit(1)
                .first(&db)
                .unwrap();
            PhotoLink {
                title: Some(format!("{}", group_year)),
                href: format!("/{}/", group_year),
                lable: Some(format!("{} images", count)),
                id: photo.id.clone(),
                size: photo.get_size(SizeTag::Small),
            }
        })
        .collect::<Vec<_>>();

    Builder::new()
        .html(|o| {
            templates::index(o, &context, "All photos", &[], &groups, &[])
        })
        .unwrap()
}

pub fn months_in_year(target_year: i32, context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade, month, year};

    let title: String = format!("Photos from {}", target_year);
    let db = context.db();
    let groups = Photo::query(context.is_authorized())
        .filter(year.eq(target_year))
        .select(sql::<(Integer, BigInt)>("month, count(*)"))
        .group_by(sql::<Integer>("month"))
        .order(sql::<Integer>("month").desc())
        .load::<(i32, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(group_month, count)| {
            let photo = Photo::query(context.is_authorized())
                .filter(year.eq(target_year))
                .filter(month.eq(group_month))
                .order((grade.desc(), date.asc()))
                .limit(1)
                .first::<Photo>(&db)
                .unwrap();

            PhotoLink {
                title: Some(monthname(group_month).to_string()),
                href: format!("/{}/{}/", target_year, group_month),
                lable: Some(format!("{} pictures", count)),
                id: photo.id.clone(),
                size: photo.get_size(SizeTag::Small),
            }
        })
        .collect::<Vec<_>>();

    if groups.is_empty() {
        not_found(&context)
    } else {
        use crate::schema::positions::dsl::{
            latitude, longitude, photo_id, positions,
        };
        let pos = Photo::query(context.is_authorized())
            .inner_join(positions)
            .filter(year.eq(target_year))
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
                templates::index(o, &context, &title, &[], &groups, &pos)
            })
            .unwrap()
    }
}

pub fn days_in_month(
    target_year: i32,
    target_month: i32,
    context: Context,
) -> Response {
    use crate::schema::photos::dsl::{date, day, grade, month, year};

    let lpath: Vec<Link> = vec![Link::year(target_year)];
    let title: String =
        format!("Photos from {} {}", monthname(target_month), target_year);
    let db = context.db();
    let groups = Photo::query(context.is_authorized())
        .filter(year.eq(target_year))
        .filter(month.eq(target_month as i32))
        .select(sql::<(Integer, BigInt)>("day, count(*)"))
        .group_by(sql::<Integer>("day"))
        .order(sql::<Integer>("day").desc())
        .load::<(i32, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(group_day, count)| {
            let photo = Photo::query(context.is_authorized())
                .filter(year.eq(target_year))
                .filter(month.eq(target_month as i32))
                .filter(day.eq(group_day as i32))
                .order((grade.desc(), date.asc()))
                .limit(1)
                .first::<Photo>(&db)
                .unwrap();

            PhotoLink {
                title: Some(format!("{}", group_day)),
                href: format!(
                    "/{}/{}/{}",
                    target_year, target_month, group_day
                ),
                lable: Some(format!("{} pictures", count)),
                id: photo.id.clone(),
                size: photo.get_size(SizeTag::Small),
            }
        })
        .collect::<Vec<_>>();

    if groups.is_empty() {
        not_found(&context)
    } else {
        use crate::schema::positions::dsl::{
            latitude, longitude, photo_id, positions,
        };
        let pos = Photo::query(context.is_authorized())
            .inner_join(positions)
            .filter(year.eq(target_year))
            .filter(month.eq(target_month as i32))
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
                templates::index(o, &context, &title, &lpath, &groups, &pos)
            })
            .unwrap()
    }
}

pub fn all_null_date(context: Context) -> Response {
    use crate::schema::photos::dsl::{date, path};

    Builder::new()
        .html(|o| {
            templates::index(
                o,
                &context,
                "Photos without a date",
                &[],
                &Photo::query(context.is_authorized())
                    .filter(date.is_null())
                    .order(path.asc())
                    .limit(500)
                    .load(&context.db())
                    .unwrap()
                    .iter()
                    .map(PhotoLink::no_title)
                    .collect::<Vec<_>>(),
                &[], // Don't care about positions here
            )
        })
        .unwrap()
}

pub fn all_for_day(
    target_year: i32,
    target_month: i32,
    target_day: i32,
    range: ImgRange,
    context: Context,
) -> Response {
    use crate::schema::photos::dsl::{day, month, year};

    let photos = Photo::query(context.is_authorized())
        .filter(year.eq(target_year))
        .filter(month.eq(target_month))
        .filter(day.eq(target_day));
    let (links, coords) = links_by_time(&context, photos, range, false);

    if links.is_empty() {
        not_found(&context)
    } else {
        Builder::new()
            .html(|o| {
                templates::index(
                    o,
                    &context,
                    &format!(
                        "Photos from {} {} {}",
                        target_day,
                        monthname(target_month),
                        target_year
                    ),
                    &[
                        Link::year(target_year),
                        Link::month(target_year, target_month),
                    ],
                    &links,
                    &coords,
                )
            })
            .unwrap()
    }
}

pub fn on_this_day(context: Context) -> Response {
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
                            lable: Some(format!("{} pictures", count)),
                            id: photo.id.clone(),
                            size: photo.get_size(SizeTag::Small),
                        }
                    })
                    .collect::<Vec<_>>(),
                &pos,
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
