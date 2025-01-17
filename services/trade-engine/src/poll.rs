use anyhow::anyhow;
use log::info;
use solana_sdk::signature::Signature;
use crate::chan::Chan;
use crate::rpc::solana_rpc_client;
use crate::trade::Trade;

pub fn poll_trade_and_cmd<F>(
    signature: Signature,
    chan: Chan,
    trade: Trade,
    log_action: String,
    handle_err: F,
) where
    F: Fn(Chan, Trade, Option<anyhow::Error>) + Send + 'static,
{
    tokio::spawn(async move {
        let rpc = solana_rpc_client();
        let res = rpc.poll_for_signature(&signature).await;
        let is_poll_err = if res.is_err() {
            Some(format!("{:?}", res))
        } else {
            let res = rpc.get_signature_status(&signature).await;
            if let Ok(Some(Ok(_))) = res {
                None
            } else {
                Some(format!("{:?}", res))
            }
        };
        if let Some(err_msg) = is_poll_err {
            let e = Some(anyhow!("could not {log_action} {:?}", err_msg));
            handle_err(chan, trade, e);
        } else {
            // trade.db_upsert_mongo()
            info!("{log_action} poll success");
        }
    });
}
