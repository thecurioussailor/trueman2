// @generated automatically by Diesel CLI.

diesel::table! {
    balances (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_id -> Uuid,
        amount -> Int8,
        locked_amount -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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
    orders (id) {
        id -> Uuid,
        user_id -> Uuid,
        market_id -> Uuid,
        #[max_length = 10]
        order_type -> Varchar,
        #[max_length = 10]
        order_kind -> Varchar,
        price -> Nullable<Int8>,
        quantity -> Int8,
        filled_quantity -> Int8,
        #[max_length = 20]
        status -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
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
    trades (id) {
        id -> Uuid,
        market_id -> Uuid,
        buyer_order_id -> Uuid,
        seller_order_id -> Uuid,
        price -> Int8,
        quantity -> Int8,
        created_at -> Timestamp,
        buyer_user_id -> Uuid,
        seller_user_id -> Uuid,
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

diesel::joinable!(balances -> tokens (token_id));
diesel::joinable!(balances -> users (user_id));
diesel::joinable!(orders -> markets (market_id));
diesel::joinable!(orders -> users (user_id));
diesel::joinable!(trades -> markets (market_id));

diesel::allow_tables_to_appear_in_same_query!(
    balances,
    markets,
    orders,
    tokens,
    trades,
    users,
);
