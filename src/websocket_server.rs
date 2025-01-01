use std::ops::Deref;
use std::sync::{Arc};
use fastwebsockets::Frame;
use futures::pin_mut;
use log::info;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use crate::chan::Chan;
use crate::cmd::InternalCommand;

pub struct WebsocketState {
    pub chan: Chan,
    pub rec:Mutex<Receiver<InternalCommand>>
}

pub async fn start_websocket_server(state:Arc<WebsocketState>) {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    while let Ok((stream,_)) = listener.accept().await {
        let state = state.clone();
        tokio::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(stream);
            let state = state.clone();
            let service = hyper::service::service_fn(move |req| {
                let state = state.clone();
                async move { server_upgrade(req, state).await }
            });
            let conn_fut = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades();
            if let Err(e) = conn_fut.await {
                println!("An error occurred: {:?}", e);
            }
        });
    }
}

async fn server_upgrade(
    mut req: hyper::Request<hyper::body::Incoming>,
    state: Arc<WebsocketState>,
) -> Result<hyper::Response<http_body_util::Empty<hyper::body::Bytes>>, fastwebsockets::WebSocketError> {
    let (response, fut) = fastwebsockets::upgrade::upgrade(&mut req)?;

    tokio::task::spawn(async move {
        if let Err(e) = tokio::task::unconstrained(handle_client(fut,state)).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    Ok(response)
}

pub async fn d(_:Frame<'_>)->Result<(),fastwebsockets::WebSocketError>{
    Ok(())
}

async fn handle_client(
    fut: fastwebsockets::upgrade::UpgradeFut,
    state: Arc<WebsocketState>,
) -> Result<(), fastwebsockets::WebSocketError> {
    // let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    let mut ws = fut.await?;
    ws.set_auto_close(true);
    let (rx, mut tx) = ws.split(tokio::io::split);
    let mut rx = fastwebsockets::FragmentCollectorRead::new(rx);




    let mut r = state.rec.lock().await;

    let ab = &mut d;

    loop {
        let a = rx.read_frame::<_, fastwebsockets::WebSocketError>(ab);
        tokio::select! {
            Some(cmd) = r.recv() => {
                match cmd {
                    InternalCommand::UpdateTrade(_)=> {
                        let w = fastwebsockets::Payload::from("abc123000".as_bytes());
                        tx.write_frame(Frame::text(w)).await.unwrap();
                    }
                    InternalCommand::Log(log) => {
                        let w = fastwebsockets::Payload::from(log.as_bytes());
                        tx.write_frame(Frame::text(w)).await.unwrap();
                    }
                    _ => {}
                }
            }
            Ok(frame) = a => {
                match frame.opcode {
                    fastwebsockets::OpCode::Close => {},
                    fastwebsockets::OpCode::Text  => {
                        let a = Vec::from(frame.payload);
                        let b = String::from_utf8(a).unwrap();
                        info!("{b}");
                        let w = fastwebsockets::Payload::from("feaea".as_bytes());
                        tx.write_frame(Frame::text(w)).await.unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
}