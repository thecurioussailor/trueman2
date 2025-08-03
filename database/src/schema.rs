// @generated automatically by Diesel CLI.

diesel::table! {
    markets (id) {
        id -> Uuid,
        #[max_length = 20]
        symbol -> Varchar,
        base_currency_id -> Uuid,
        quote_currency_id -> Uuid,
        min_order_size -> Int8,
        tick_size -> Int8,
        is_active -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tokens (id) {
        id -> Uuid,
        #[max_length = 10]
        symbol -> Varchar,
        #[max_length = 50]
        name -> Varchar,
        decimals -> Int4,
        is_active -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Varchar,
        password_hash -> Text,
        is_admin -> Bool,
        created_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    markets,
    tokens,
    users,
);
