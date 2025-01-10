use crate::chan_trade::{InterestedTx};
use crate::jito_chan::TipStatistics;
use crate::trade22::{Trade, TradePrice, TradeResponseResult};



#[derive(Debug,Clone)]
pub enum BroadcastCommand {

    LogTrade(Trade),
    TradeUpdate {
        trade: Trade,
        interested_tx: InterestedTx
    },
    RaydiumInit(InterestedTx),
    IsPumpTrade,
    DoNothing,
    PumpPrice,
    ExternalTrade(Trade),
    Log(String),

    UpdateTrade(Trade),
    InsertTrade(Trade),
    UpdatePrice(TradePrice),
    InsertPrice(TradePrice),

    WatchPool(String),
    PriceTvl(TradePrice),
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

#[derive(Debug)]
pub enum InternalCommand {
    DslResponse(String),
    Dsl(String,String,tokio::sync::oneshot::Sender<InternalCommand>),
    RpcTradeResponse(Result<Trade,Trade>),
    PumpSwapMaybe(InterestedTx),
    JitoTip(TipStatistics),
    SellTradeSuccess(Trade),
    BuyTradeSuccess(Trade),
    LogTrade(Trade),
    TradeUpdate {
        trade: Trade,
        interested_tx: InterestedTx
    },
    RaydiumInit(InterestedTx),
    IsPumpTrade,
    DoNothing,
    PumpPrice,
    ExternalTrade(Trade),
    Log(String),

    UpdateTrade(Trade),
    InsertTrade(Trade),
    UpdatePrice(TradePrice),
    InsertPrice(TradePrice),

    WatchPool(String),
    PriceTvl(TradePrice),
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