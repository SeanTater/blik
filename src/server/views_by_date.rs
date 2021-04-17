use super::splitlist::links_by_time;
use super::{not_found, redirect_to_img, Context, ImgRange, Link, PhotoLink};
use crate::models::{Photo, SizeTag};
use crate::templates::{self, RenderRucte};
use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::{Datelike, Duration, Local};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable};
use log::warn;
use serde::Deserialize;
use warp::http::response::Builder;
use warp::reply::Response;

pub fn all_years(context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade};
    let db = context.db().unwrap();
    let groups = Photo::query(context.is_authorized())
        .select(sql::<(Nullable<Integer>, BigInt)>(
            "cast(strftime('%Y', date) as int) y, count(*)",
        ))
        .group_by(sql::<Nullable<Integer>>("y"))
        .order(sql::<Nullable<Integer>>("y").desc())
        .load::<(Option<i32>, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(year, count)| {
            let q = Photo::query(context.is_authorized())
                .order((grade.desc(), date.asc()))
                .limit(1);
            let photo = if let Some(year) = year {
                q.filter(date.ge(start_of_year(year)))
                    .filter(date.lt(start_of_year(year + 1)))
            } else {
                q.filter(date.is_null())
            };
            let photo = photo.first::<Photo>(&db).unwrap();
            PhotoLink {
                title: Some(
                    year.map(|y| format!("{}", y))
                        .unwrap_or_else(|| "-".to_string()),
                ),
                href: format!("/{}/", year.unwrap_or(0)),
                lable: Some(format!("{} images", count)),
                id: photo.id,
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

fn start_of_year(year: i32) -> NaiveDateTime {
    NaiveDate::from_ymd(year, 1, 1).and_hms(0, 0, 0)
}

pub fn months_in_year(year: i32, context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade};

    let title: String = format!("Photos from {}", year);
    let db = context.db().unwrap();
    let groups = Photo::query(context.is_authorized())
        .filter(date.ge(start_of_year(year)))
        .filter(date.lt(start_of_year(year + 1)))
        .select(sql::<(Integer, BigInt)>(
            "cast(strftime('%M', date) as int) m, count(*)",
        ))
        .group_by(sql::<Integer>("m"))
        .order(sql::<Integer>("m").desc())
        .load::<(i32, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(month, count)| {
            let month = month as u32;
            let photo = Photo::query(context.is_authorized())
                .filter(date.ge(start_of_month(year, month)))
                .filter(date.lt(start_of_month(year, month + 1)))
                .order((grade.desc(), date.asc()))
                .limit(1)
                .first::<Photo>(&db)
                .unwrap();

            PhotoLink {
                title: Some(monthname(month).to_string()),
                href: format!("/{}/{}/", year, month),
                lable: Some(format!("{} pictures", count)),
                id: photo.id,
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
            .filter(date.ge(start_of_year(year)))
            .filter(date.lt(start_of_year(year + 1)))
            .select((photo_id, latitude, longitude))
            .load(&db)
            .map_err(|e| warn!("Failed to load positions: {}", e))
            .unwrap_or_default()
            .into_iter()
            .map(|(p_id, lat, long): (i32, i32, i32)| {
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

fn start_of_month(year: i32, month: u32) -> NaiveDateTime {
    let date = if month > 12 {
        NaiveDate::from_ymd(year + 1, month - 12, 1)
    } else {
        NaiveDate::from_ymd(year, month, 1)
    };
    date.and_hms(0, 0, 0)
}

pub fn days_in_month(year: i32, month: u32, context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade};

    let lpath: Vec<Link> = vec![Link::year(year)];
    let title: String = format!("Photos from {} {}", monthname(month), year);
    let db = context.db().unwrap();
    let groups = Photo::query(context.is_authorized())
        .filter(date.ge(start_of_month(year, month)))
        .filter(date.lt(start_of_month(year, month + 1)))
        .select(sql::<(Integer, BigInt)>(
            "cast(strftime('%D', date) as int) d, count(*)",
        ))
        .group_by(sql::<Integer>("d"))
        .order(sql::<Integer>("d").desc())
        .load::<(i32, i64)>(&db)
        .unwrap()
        .iter()
        .map(|&(day, count)| {
            let day = day as u32;
            let fromdate =
                NaiveDate::from_ymd(year, month, day).and_hms(0, 0, 0);
            let photo = Photo::query(context.is_authorized())
                .filter(date.ge(fromdate))
                .filter(date.lt(fromdate + Duration::days(1)))
                .order((grade.desc(), date.asc()))
                .limit(1)
                .first::<Photo>(&db)
                .unwrap();

            PhotoLink {
                title: Some(format!("{}", day)),
                href: format!("/{}/{}/{}", year, month, day),
                lable: Some(format!("{} pictures", count)),
                id: photo.id,
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
            .filter(date.ge(start_of_month(year, month)))
            .filter(date.lt(start_of_month(year, month + 1)))
            .select((photo_id, latitude, longitude))
            .load(&db)
            .map_err(|e| warn!("Failed to load positions: {}", e))
            .unwrap_or_default()
            .into_iter()
            .map(|(p_id, lat, long): (i32, i32, i32)| {
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
                    .load(&context.db().unwrap())
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
    year: i32,
    month: u32,
    day: u32,
    range: ImgRange,
    context: Context,
) -> Response {
    let thedate = NaiveDate::from_ymd(year, month, day).and_hms(0, 0, 0);
    use crate::schema::photos::dsl::date;

    let photos = Photo::query(context.is_authorized())
        .filter(date.ge(thedate))
        .filter(date.lt(thedate + Duration::days(1)));
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
                        day,
                        monthname(month),
                        year
                    ),
                    &[Link::year(year), Link::month(year, month)],
                    &links,
                    &coords,
                )
            })
            .unwrap()
    }
}

pub fn on_this_day(context: Context) -> Response {
    use crate::schema::photos::dsl::{date, grade};
    use crate::schema::positions::dsl::{
        latitude, longitude, photo_id, positions,
    };

    let (month, day) = {
        let today = Local::now();
        (today.month(), today.day())
    };
    let db = context.db().unwrap();
    let pos = Photo::query(context.is_authorized())
        .inner_join(positions)
        .filter(
            sql("extract(month from date)=").bind::<Integer, _>(month as i32),
        )
        .filter(sql("extract(day from date)=").bind::<Integer, _>(day as i32))
        .select((photo_id, latitude, longitude))
        .load(&db)
        .map_err(|e| warn!("Failed to load positions: {}", e))
        .unwrap_or_default()
        .into_iter()
        .map(|(p_id, lat, long): (i32, i32, i32)| ((lat, long).into(), p_id))
        .collect::<Vec<_>>();

    Builder::new()
        .html(|o| {
            templates::index(
                o,
                &context,
                &format!("Photos from {} {}", day, monthname(month)),
                &[],
                &Photo::query(context.is_authorized())
                    .select(sql::<(Integer, BigInt)>(
                        "cast(extract(year from date) as int) y, count(*)",
                    ))
                    .group_by(sql::<Integer>("y"))
                    .filter(
                        sql("extract(month from date)=")
                            .bind::<Integer, _>(month as i32),
                    )
                    .filter(
                        sql("extract(day from date)=")
                            .bind::<Integer, _>(day as i32),
                    )
                    .order(sql::<Integer>("y").desc())
                    .load::<(i32, i64)>(&db)
                    .unwrap()
                    .iter()
                    .map(|&(year, count)| {
                        let fromdate =
                            NaiveDate::from_ymd(year, month as u32, day)
                                .and_hms(0, 0, 0);
                        let photo = Photo::query(context.is_authorized())
                            .filter(date.ge(fromdate))
                            .filter(date.lt(fromdate + Duration::days(1)))
                            .order((grade.desc(), date.asc()))
                            .limit(1)
                            .first::<Photo>(&db)
                            .unwrap();

                        PhotoLink {
                            title: Some(format!("{}", year)),
                            href: format!("/{}/{}/{}", year, month, day),
                            lable: Some(format!("{} pictures", count)),
                            id: photo.id,
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
    let db = context.db().unwrap();
    if let Some(from_date) = date_of_img(&db, param.from) {
        let q = Photo::query(context.is_authorized())
            .select(id)
            .filter(
                date.gt(from_date)
                    .or(date.eq(from_date).and(id.gt(param.from))),
            )
            .order((date, id));
        if let Ok(photo) = q.first::<i32>(&db) {
            return redirect_to_img(photo);
        }
    }
    not_found(&context)
}

pub fn prev_image(context: Context, param: FromParam) -> Response {
    use crate::schema::photos::dsl::{date, id};
    let db = context.db().unwrap();
    if let Some(from_date) = date_of_img(&db, param.from) {
        let q = Photo::query(context.is_authorized())
            .select(id)
            .filter(
                date.lt(from_date)
                    .or(date.eq(from_date).and(id.lt(param.from))),
            )
            .order((date.desc(), id.desc()));
        if let Ok(photo) = q.first::<i32>(&db) {
            return redirect_to_img(photo);
        }
    }
    not_found(&context)
}

#[derive(Deserialize)]
pub struct FromParam {
    from: i32,
}

pub fn date_of_img(db: &SqliteConnection, photo_id: i32) -> Option<NaiveDateTime> {
    use crate::schema::photos::dsl::{date, photos};
    photos.find(photo_id).select(date).first(db).unwrap_or(None)
}

pub fn monthname(n: u32) -> &'static str {
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
