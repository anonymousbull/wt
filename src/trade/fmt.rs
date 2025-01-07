use std::fmt;
use colored::Colorize;
use rust_decimal_macros::dec;
use crate::trade22::{Trade, TradeBuySuccessLog, TradePool};




impl fmt::Display for Trade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.id;
        let state = self.state;
        let signature = self.rpc_logs.iter().find_map(|x|{
            x.success_signature()
        }).map(|x|format!("https://solscan.io/tx/{x}")).unwrap();
        let amm = match self.amm {
            TradePool::RayAmm4(_) => {
                unimplemented!()
            }
            TradePool::PumpBondingCurve(_) => {
                format!("https://pump.fun/coin/{}",self.mint().to_string())
            }
        };
        let pct = self.pct;
        write!(f, "id={id} state={state:?} signature={signature} amm={amm} pct={pct}")
    }
}