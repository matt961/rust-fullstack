// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        id -> Uuid,
        user_id -> Int4,
        post_content -> Text,
        tags -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 320]
        email -> Varchar,
    }
}

diesel::joinable!(posts -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(posts, users,);
