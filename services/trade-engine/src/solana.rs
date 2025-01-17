use rust_decimal::Decimal;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

const MICRO_LAMPORTS_PER_LAMPORT: u64 = 1_000_000;

pub fn lamports_per_sol_dec() ->Decimal{
    Decimal::from(LAMPORTS_PER_SOL)
}

pub fn micro_lamports_per_sol_dec() ->Decimal{
    Decimal::from(LAMPORTS_PER_SOL * MICRO_LAMPORTS_PER_LAMPORT)
}

pub fn lamports_to_sol_dec(n:Decimal) ->Decimal{
    n / Decimal::from(LAMPORTS_PER_SOL)
}

