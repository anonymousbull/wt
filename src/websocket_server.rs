use crate::chan_type::Chan;
use crate::cmd::{BroadcastCommand, InternalCommand};
use crate::trade22::TradeState;
use fastwebsockets::Frame;
use log::info;
use std::hash::Hasher;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use futures::pin_mut;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{oneshot, Mutex};

pub struct WebsocketState {
    pub chan: Chan,
    pub rec: tokio::sync::broadcast::Sender<BroadcastCommand>,
}

pub async fn start_websocket_server(state: Arc<WebsocketState>) {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    info!("listening 8080");
    while let Ok((stream, _)) = listener.accept().await {
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
) -> Result<
    hyper::Response<http_body_util::Empty<hyper::body::Bytes>>,
    fastwebsockets::WebSocketError,
> {
    let (response, fut) = fastwebsockets::upgrade::upgrade(&mut req)?;

    tokio::task::spawn(async move {
        if let Err(e) = tokio::task::unconstrained(handle_client(fut, state)).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    Ok(response)
}

pub async fn d(_: Frame<'_>) -> Result<(), fastwebsockets::WebSocketError> {
    Ok(())
}

async fn handle_client(
    fut: fastwebsockets::upgrade::UpgradeFut,
    state: Arc<WebsocketState>,
) -> Result<(), fastwebsockets::WebSocketError> {
    let mut user_sk = None;
    info!("hello");
    // let mut ws = fastwebsockets::FragmentCollector::new(fut.await?);
    let mut ws = fut.await?;
    ws.set_auto_close(true);
    let (rx, mut tx) = ws.split(tokio::io::split);
    let mut rx = fastwebsockets::FragmentCollectorRead::new(rx);

    let mut r = state.rec.subscribe();

    let ab = &mut d;



    loop {
        let a = rx.read_frame::<_, fastwebsockets::WebSocketError>(ab);
        let mut aa = || async {
            if let Ok(frame) = a.await {
                match frame.opcode {
                    fastwebsockets::OpCode::Close => {},
                    fastwebsockets::OpCode::Text  => {
                        let a = Vec::from(frame.payload);
                        let mut message = String::from_utf8(a).unwrap();
                        if user_sk.is_none() {
                            let id = message.lines().next().unwrap();
                            if id.len() == 32 {
                                user_sk = Some(id.to_string());
                                let w = fastwebsockets::Payload::from("Welcome".as_bytes());
                                tx.write_frame(Frame::text(w)).await.unwrap();
                            }
                        } else if let Some(ref user_sk) = user_sk {
                            let (s, r) = oneshot::channel::<InternalCommand>();
                            message.insert_str(0, &format!("user_sk={} ",user_sk));
                            state.chan.dsl.try_send(InternalCommand::Dsl(user_sk.clone(),message,s)).unwrap();

                            match tokio::time::timeout(Duration::from_secs(10), r).await {
                                Ok(Ok(InternalCommand::DslResponse(message))) => {
                                    let w = fastwebsockets::Payload::from(message.as_bytes());
                                    tx.write_frame(Frame::text(w)).await.unwrap();
                                }
                                Ok(Err(_)) => {
                                    println!("The sender was dropped before sending the message.");
                                }
                                e => {
                                    println!("Timed out waiting for the message. {:?}",e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        };

        tokio::select! {
            Ok(cmd) = r.recv() => {
                match cmd {
                    _ => {}
                }
            }
            _ = aa() => {

            }
        }
    }
}

pub fn process_command_message(message: &str, chan: Chan) -> Vec<String> {
    let mut responses = Vec::new();

    // Split the message into whitespace-separated commands
    let commands: Vec<&str> = message.trim().split_whitespace().collect();

    for command in commands {
        // Commands must start with "sl", followed by a number
        if command.starts_with("sl") {
            // Attempt to parse the number following "sl"
            if let Ok(value) = command[2..].parse::<i32>() {
                // Successfully parsed; process the command
                responses.push(format!("Processed 'sl' with value: {}", value));
            } else {
                // Error: Invalid number format
                responses.push(format!("Error: Invalid 'sl' command '{}'", command));
            }
        } else {
            // Error: Unknown command
            responses.push(format!("Error: Unknown command '{}'", command));
        }
    }

    responses
}
