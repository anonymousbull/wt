use crate::price::Price;
use crate::trade::Trade;

#[derive(Debug)]
pub enum InternalCommand {
    UpdateTrade(Trade),
    InsertTrade(Trade),
    UpdatePrice(Price),
    InsertPrice(Price),
    StopWatchTrade(Trade),
    WatchPool(String),
    PriceTvl(Price),
    OnTrading,
    OffTrading,
    // UnconfirmedPosition {
    //     data:IxPositionUnconfirmed
    // },
    // MaybeConfirmedPosition {
    //     data: Vec<IxAccount<SwapIxAmm>>
    // },
    // Test {
    // },
    // SolanaBlock {
    //     data: solana_client::rpc_response::Response<RpcBlockUpdate>
    // },
    // BgSaveAssets {
    //     data:Vec<Asset>
    // },
    // PositionUpdateRequest {
    //     data:Vec<Asset>
    // },
    // PositionRequest {
    //     data: PositionRequest,
    //     rtn: Option<oneshot::Sender<InternalCommand>>
    // },
    // PositionResponse {
    //     data: Position,
    // },
    // TradeCheckRequest,
    // TestTradeRequest {
    //     rtn: oneshot::Sender<InternalCommand>,
    // },
    // TestTradeResponse {
    //     data: Position,
    // },
    // BgSavePrices {
    //     data: Vec<Price>
    // },
    // BgSaveSwapIxs {
    //     data: Vec<SwapIx>
    // },
    //
    // RedisMemecoinsUpdateRequest {
    //     data: Vec<Memecoin>,
    // },
    // InsertMemecoinsToPostgresRequest {
    //     data: Vec<Memecoin>,
    // },
    // GetMemecoinsByManyPoolsRequest {
    //     data: Vec<RaydiumSwapDto>,
    //     rtn: Sender<InternalCommand>,
    // },
    //
    // TryInsertPriceAlertRequest {
    //     data: Vec<Memecoin>,
    //     rtn: Sender<InternalCommand>,
    // },
    // TryInsertPriceAlertResponse {
    //     data: Option<Vec<PriceAlert>>,
    // },
    // GetMintsByPoolRequest {
    //     data: String,
    //     rtn: Sender<InternalCommand>,
    // },
    // GetMintsByPoolResponse {
    //     data: anyhow::Result<MintsByPool>,
    // },
    // UpdatedMemecoins {
    //     data: Vec<Memecoin>,
    // },
    // InsertUserRequest {
    //     data: User,
    //     rtn: Sender<InternalCommand>,
    // },
    // InsertUserResponse {
    //     data: User,
    // },
    // GetUserRequest {
    //     data: String,
    //     rtn: Sender<InternalCommand>,
    // },
    // GetUserResponse {
    //     data: User,
    // },
    //
    // UpsertMemecoinsResponse {
    //     data: Vec<Memecoin>,
    // },
    // GetDecodedMemecoinRequest {
    //     base_mint: String,
    //     quote_mint: String,
    //     pool_vault1: String,
    //     pool_vault2: String,
    //     pool: String,
    //     rtn: Sender<InternalCommand>,
    // },
    // RequestLatestMemecoins {
    //     rtn: Sender<InternalCommand>,
    // },
    // ReturnLatestMemecoins {
    //     data: Vec<Memecoin>,
    // },
    // NewRaydiumSwap {
    //     data: SwapIx,
    // },
    // NewRaydiumSwapSelf {
    //     data: SwapIx,
    // },
    // NewRaydiumPool {
    //     data: PoolIx
    // },
    // NewRaydiumSwapBatch {
    //     data: Vec<SwapIx>,
    // },
}