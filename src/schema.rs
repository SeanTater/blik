table! {
    attributions (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    people (id) {
        id -> Integer,
        slug -> Text,
        person_name -> Text,
    }
}

table! {
    photo_people (id) {
        id -> Integer,
        photo_id -> Integer,
        person_id -> Integer,
    }
}

table! {
    photo_places (id) {
        id -> Integer,
        photo_id -> Integer,
        place_id -> Integer,
    }
}

table! {
    photo_tags (id) {
        id -> Integer,
        photo_id -> Integer,
        tag_id -> Integer,
    }
}

table! {
    photos (id) {
        id -> Integer,
        path -> Text,
        date -> Nullable<Timestamp>,
        year -> Integer,
        month -> Integer,
        day -> Integer,
        grade -> Nullable<SmallInt>,
        rotation -> SmallInt,
        is_public -> Bool,
        attribution_id -> Nullable<Integer>,
        width -> Integer,
        height -> Integer,
    }
}

table! {
    places (id) {
        id -> Integer,
        slug -> Text,
        place_name -> Text,
        osm_id -> Nullable<BigInt>,
        osm_level -> Nullable<SmallInt>,
    }
}

table! {
    positions (id) {
        id -> Integer,
        photo_id -> Integer,
        latitude -> Integer,
        longitude -> Integer,
    }
}

table! {
    tags (id) {
        id -> Integer,
        slug -> Text,
        tag_name -> Text,
    }
}

joinable!(photo_people -> people (person_id));
joinable!(photo_people -> photos (photo_id));
joinable!(photo_places -> photos (photo_id));
joinable!(photo_places -> places (place_id));
joinable!(photo_tags -> photos (photo_id));
joinable!(photo_tags -> tags (tag_id));
joinable!(positions -> photos (photo_id));

allow_tables_to_appear_in_same_query!(
    attributions,
    people,
    photo_people,
    photo_places,
    photo_tags,
    photos,
    places,
    positions,
    tags,
);
