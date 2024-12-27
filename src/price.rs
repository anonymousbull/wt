use crate::constant::pg_conn;
use crate::implement_diesel;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel_async::pooled_connection::deadpool::Object;
use diesel_async::RunQueryDsl;
use raydium_amm::solana_program::native_token::LAMPORTS_PER_SOL;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::prices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Price {
    pub id: i64,
    pub trade_id: String,
    pub price: Decimal,
    pub tvl: Decimal,
}

implement_diesel!(Price, prices);


#[derive(Debug, Clone)]
pub struct PriceBuilder {
    pub price: Decimal,
    pub tvl: Decimal,
}

impl PriceBuilder {
    pub fn build(self, id:i64, trade_id:String) -> Price {
        Price {
            id,
            trade_id,
            price: self.price,
            tvl: self.tvl,
        }
    }
}

pub fn get_price_tvl(sol_amount: u64, token_amount: u64, token_decimals: u8) -> PriceBuilder {
    // 1000
    let token_amount_normal =
        Decimal::from(token_amount) / Decimal::from(10u64.pow(token_decimals as u32));
    // 1
    let sol_amount_normal = Decimal::from(sol_amount) / Decimal::from(LAMPORTS_PER_SOL);
    // 1 MEMECOIN = 0.0001 SOL
    let price = sol_amount_normal / token_amount_normal;
    let tvl = (price * token_amount_normal) + sol_amount_normal;
    PriceBuilder { price, tvl }
}