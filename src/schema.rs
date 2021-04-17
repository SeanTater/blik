table! {
    attributions (id) {
        id -> Nullable<Integer>,
        name -> Text,
    }
}

table! {
    cameras (id) {
        id -> Nullable<Integer>,
        manufacturer -> Text,
        model -> Text,
    }
}

table! {
    people (id) {
        id -> Nullable<Integer>,
        slug -> Text,
        person_name -> Text,
    }
}

table! {
    photo_people (id) {
        id -> Nullable<Integer>,
        photo_id -> Integer,
        person_id -> Integer,
    }
}

table! {
    photo_places (id) {
        id -> Nullable<Integer>,
        photo_id -> Integer,
        place_id -> Integer,
    }
}

table! {
    photo_tags (id) {
        id -> Nullable<Integer>,
        photo_id -> Integer,
        tag_id -> Integer,
    }
}

table! {
    photos (id) {
        id -> Nullable<Integer>,
        path -> Text,
        date -> Nullable<Text>,
        grade -> Nullable<Integer>,
        rotation -> Integer,
        is_public -> Nullable<Integer>,
        camera_id -> Nullable<Integer>,
        attribution_id -> Nullable<Integer>,
        width -> Integer,
        height -> Integer,
    }
}

table! {
    places (id) {
        id -> Nullable<Integer>,
        slug -> Text,
        place_name -> Text,
        osm_id -> Nullable<Integer>,
        osm_level -> Nullable<Integer>,
    }
}

table! {
    positions (id) {
        id -> Nullable<Integer>,
        photo_id -> Integer,
        latitude -> Integer,
        longitude -> Integer,
    }
}

table! {
    tags (id) {
        id -> Nullable<Integer>,
        slug -> Text,
        tag_name -> Text,
    }
}

table! {
    users (id) {
        id -> Nullable<Integer>,
        username -> Text,
        password -> Text,
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
    cameras,
    people,
    photo_people,
    photo_places,
    photo_tags,
    photos,
    places,
    positions,
    tags,
    users,
);
