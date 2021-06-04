table! {
    annotation (media_id, name, top, bottom, left, right, details) {
        media_id -> Text,
        name -> Text,
        top -> Integer,
        bottom -> Integer,
        left -> Integer,
        right -> Integer,
        details -> Text,
    }
}

table! {
    media (id) {
        id -> Text,
        path -> Text,
        date -> Nullable<Timestamp>,
        rotation -> SmallInt,
        is_public -> Bool,
        width -> Integer,
        height -> Integer,
        story -> Text,
        lat -> Nullable<Double>,
        lon -> Nullable<Double>,
        make -> Nullable<Text>,
        model -> Nullable<Text>,
        caption -> Nullable<Text>,
        mimetype -> Text,
    }
}

table! {
    story (name) {
        name -> Text,
        title -> Text,
        description -> Text,
        created_on -> Timestamp,
        last_updated -> Timestamp,
        latest_media -> Nullable<Text>,
        media_count -> Integer,
    }
}

table! {
    thumbnail (id) {
        id -> Text,
        content -> Binary,
        mimetype -> Text,
    }
}

joinable!(annotation -> media (media_id));
joinable!(story -> media (latest_media));

allow_tables_to_appear_in_same_query!(
    annotation,
    media,
    story,
    thumbnail,
);
