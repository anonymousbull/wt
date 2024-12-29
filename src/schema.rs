// @generated automatically by Diesel CLI.

diesel::table! {
    trade_prices (id) {
        id -> Int8,
        trade_id -> Text,
        price -> Numeric,
        tvl -> Numeric,
    }
}

diesel::table! {
    trades (id) {
        id -> Text,
        coin_vault -> Text,
        pc_vault -> Text,
        coin_mint -> Text,
        pc_mint -> Text,
        decimals -> Int2,
        token_program_id -> Text,
        amount -> Numeric,
        entry_time -> Nullable<Timestamptz>,
        entry_price -> Nullable<Numeric>,
        exit_time -> Nullable<Timestamptz>,
        exit_price -> Nullable<Numeric>,
        pct -> Numeric,
        state -> Int4,
        tx_in_id -> Nullable<Text>,
        tx_out_id -> Nullable<Text>,
        sol_before -> Numeric,
        sol_after -> Nullable<Numeric>,
        root_kp -> Bytea,
        pool_id -> Text,
        user_wallet -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    trade_prices,
    trades,
);
