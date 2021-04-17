use anyhow::Result;
use crate::schema::people::dsl::people;
use crate::schema::photos::dsl::photos;
use crate::schema::places::dsl::places;
use crate::schema::tags::dsl::tags;
use diesel::expression::dsl::{count_star, sql};
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable};

pub fn show_stats(db: &SqliteConnection) -> Result<()> {
    println!(
        "There are {} photos in total.",
        photos.select(count_star()).first::<i64>(db)?,
    );

    println!(
        "There are {} persons, {} places, and {} tags mentioned.",
        people.select(count_star()).first::<i64>(db)?,
        places.select(count_star()).first::<i64>(db)?,
        tags.select(count_star()).first::<i64>(db)?,
    );

    // Something like this should be possible, I guess?
    //
    // use schema::photos::dsl::date;
    // let year = date_part("year", date).aliased("y");
    // println!("Count per year: {:?}",
    //          photos.select((year, count_star()))
    //              .group_by(year)
    //              .limit(10)
    //              .load::<(Option<f64>, i64)>(db));

    println!(
        "Count per year: {:?}",
        photos
            .select(sql::<(Nullable<Double>, BigInt)>(
                "strftime('%Y', date) y, count(*)"
            ))
            .group_by(sql::<Nullable<Double>>("y"))
            .order(sql::<Nullable<Double>>("y").desc())
            .load::<(Option<f64>, i64)>(db)?
            .iter()
            .map(|&(y, n)| format!("{}: {}", y.unwrap_or(0.0), n))
            .collect::<Vec<_>>(),
    );

    Ok(())
}
