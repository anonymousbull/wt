use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use raydium_amm::solana_program::instruction::Instruction;

#[derive(Clone,Serialize,Deserialize,Debug, PartialEq)]
pub enum TradeTransaction {
    Jito(Transaction),
    General(Transaction)
}

impl TradeTransaction {
    pub fn transaction(&self) ->&Transaction{
        match self {
            TradeTransaction::Jito(v) => v,
            TradeTransaction::General(v) => v
        }
    }
}


