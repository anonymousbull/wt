use std::collections::HashMap;
use solana_sdk::signature::Keypair;
use tokio::sync::oneshot::Sender;
use raydium_amm::solana_program::pubkey::Pubkey;
use crate::event::InternalCommand;
use crate::trade::Trade;

pub struct Cache {
    pub trades: HashMap<i64, Trade>,
    pub mints: HashMap<Pubkey, HashMap<i64,Trade>>,
    pub oneshots: HashMap<i64, tokio::sync::oneshot::Sender<InternalCommand>>,
}

impl Cache {
    pub fn upsert(&mut self, trade: &Trade) {
        self.trades.insert(trade.id, trade.clone());
        self.mints
            .entry(trade.mint())
            .or_insert_with(HashMap::new)
            .insert(trade.id,trade.clone());
    }
    pub fn remove(&mut self, trade: &Trade) {
        if let Some(trade) = self.trades.remove(&trade.id) {
            self.mints.entry(trade.mint()).and_modify(|x| {
                x.remove(&trade.id);
            });
            if let Some(array) = self.mints.get(&trade.mint()) {
                if array.is_empty() {
                    self.mints.remove(&trade.mint());
                }
            }
        }
    }
    pub fn get_by_id(&self, id: i64) -> Option<&Trade> {
        self.trades.get(&id)
    }
    pub fn get_by_trade(&self, trade: &Trade) -> Option<&Trade> {
        self.trades.get(&trade.id)
    }
    pub fn get_trades_by_mint(&self, mint: &Pubkey) -> Option<Vec<&Trade>> {
        self.mints
            .get(&mint)
            .and_then(|x| Some(x.values().collect::<Vec<_>>()))
    }
    pub fn values(&self) -> Vec<Trade> {
        self.trades.values().cloned().collect()
    }
}
