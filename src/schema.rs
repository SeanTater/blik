table! {
    attributions (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    photo_tags (id) {
        id -> Integer,
        photo_id -> Text,
        tag_id -> Integer,
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
        attribution_id -> Nullable<Integer>,
        width -> Integer,
        height -> Integer,
        thumbnail -> Binary,
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
    tags (id) {
        id -> Integer,
        slug -> Text,
        tag_name -> Text,
    }
}

joinable!(photo_tags -> photos (photo_id));
joinable!(photo_tags -> tags (tag_id));

allow_tables_to_appear_in_same_query!(
    attributions,
    photo_tags,
    photos,
    story,
    tags,
);
