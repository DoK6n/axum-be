// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int8,
        username -> Text,
        created_at -> Timestamptz,
    }
}
