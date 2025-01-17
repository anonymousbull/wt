use serde::{Deserialize, Serialize};
use raydium_amm::solana_program::instruction::{AccountMeta, Instruction};
use raydium_amm::solana_program::pubkey::Pubkey;

#[derive(Clone,Serialize,Deserialize,Debug)]
pub enum TradeInstruction {
    Jito(Vec<Instruction>),
    General(Vec<Instruction>)
}


impl TradeInstruction {
    pub fn instructions(&self)->&Vec<Instruction>{
        match self {
            TradeInstruction::Jito(v) => v,
            TradeInstruction::General(v) => v
        }
    }
}

fn build_memo_instruction() -> Instruction {
    let trader_apimemo_program =
        Pubkey::from_str_const("HQ2UUt18uJqKaQFJhgV9zaTdQxUZjNrsKFgoEDquBkcx");
    let accounts = vec![AccountMeta::new(trader_apimemo_program, false)];
    let bx_memo_marker_msg = String::from("Powered by bloXroute Trader Api")
        .as_bytes()
        .to_vec();
    Instruction {
        program_id: trader_apimemo_program,
        accounts,
        data: bx_memo_marker_msg,
    }
}