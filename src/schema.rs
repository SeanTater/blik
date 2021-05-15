table! {
    annotation (photo_id, name, top, bottom, left, right, details) {
        photo_id -> Text,
        name -> Text,
        top -> Integer,
        bottom -> Integer,
        left -> Integer,
        right -> Integer,
        details -> Text,
    }
}

table! {
    photos (id) {
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
    }
}

table! {
    story (name) {
        name -> Text,
        description -> Text,
        created_on -> Timestamp,
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
