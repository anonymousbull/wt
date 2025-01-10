

use crate::cmd::{BroadcastCommand, InternalCommand};

#[derive(Clone)]
pub struct Chan {
    // pub memecoin: Sender<InternalCommand>,
    // pub memecoin_tick: Sender<InternalCommand>,
    // pub pa: Sender<InternalCommand>,
    // pub app: tokio::sync::mpsc::UnboundedSender<InternalCommand>,
    pub bg: tokio::sync::mpsc::Sender<InternalCommand>,
    pub dsl: tokio::sync::mpsc::Sender<InternalCommand>,
    // pub user: Sender<InternalCommand>,
    // pub http: tokio::sync::mpsc::UnboundedSender<InternalCommand>,
    pub trade: tokio::sync::mpsc::Sender<InternalCommand>,
    pub ws: tokio::sync::broadcast::Sender<BroadcastCommand>,
}