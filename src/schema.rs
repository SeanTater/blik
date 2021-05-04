table! {
    annotation (photo_id, name, details) {
        photo_id -> Text,
        name -> Text,
        details -> Text,
    }
}

table! {
    photos (id) {
        id -> Text,
        path -> Text,
        date -> Nullable<Timestamp>,
        year -> Integer,
        month -> Integer,
        day -> Integer,
        grade -> Nullable<SmallInt>,
        rotation -> SmallInt,
        is_public -> Bool,
        width -> Integer,
        height -> Integer,
        story -> Text,
    }
}

table! {
    story (name) {
        name -> Text,
        description -> Nullable<Text>,
        created_on -> Nullable<Timestamp>,
    }
}

table! {
    thumbnail (id) {
        id -> Text,
        content -> Binary,
    }
}

joinable!(annotation -> photos (photo_id));

allow_tables_to_appear_in_same_query!(
    annotation,
    photos,
    story,
    thumbnail,
);
