use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use chrono::Utc;
use log::info;
use ollama_rs::Ollama;
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use schemars::{schema_for, Schema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::signer::Signer;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::chan_type::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{mongo, pg_conn};
use crate::rpc::{rpc1, rpc1_unchecked};
use crate::trade22::{Trade};
use crate::type_dsl::*;
use crate::user_type::User;

fn get_dsl_fns() -> Vec<(String, Schema)> {
    vec![
        ("Config".to_string(),schema_for!(CfgPrompt)),
        ("Balance".to_string(),schema_for!(BalancePrompt)),
    ].iter().map(|x| (x.0.clone(),x.1.clone())).collect::<Vec<_>>()
}

fn get_all_tools() -> Vec<ToolCall> {
    get_dsl_fns().into_iter().map(|(name,x)|ToolCall{
        function: ToolCallFunction{
            name: name.to_string(),
            flattened_value: x.to_value()
        },
        kind: "function".to_string(),
    }).collect::<Vec<_>>()
}


pub async fn dsl_chan(chan:Chan,mut rec: Receiver<InternalCommand>) {
    let client = Client::new();
    let pg = pg_conn().await;
    let mon = mongo().await;
    let users = mon.collection::<User>("users");
    while let Some(message) = rec.recv().await {
        match message {
            InternalCommand::Dsl(id,dsl,s) => {

                let mut message = ChatMessage{
                    stream:false,
                    model: "llama3.2".to_string(),
                    messages: vec![
                        Message{
                        role: MessageRole::System,
                        content: "You are a Solana Defi Degen king".to_string(),
                    },
                        Message{
                            role: MessageRole::User,
                            content: dsl,
                        }
                    ],
                    tools: get_all_tools(),
                };

                let mut a = client.post("http://localhost:11434/api/chat")
                    .json(&message)
                    .send()
                    .await
                    .unwrap()
                    .json::<ChatMessageResponse>()
                    .await
                    .unwrap();

                for x in a.message.tool_calls {
                    match x.function {
                        PromptResponse::Config(data) => {
                            let sk = id.clone();
                            let mut user = User::get_by_sk(sk.as_str(),&users).await.unwrap();
                            if let Some(x) = data.tp {
                                user.trade.cfg.tp = Decimal::from_f64(x).unwrap();
                            }
                            if let Some(x) = data.sl {
                                user.trade.cfg.sl = Decimal::from_f64(x).unwrap();
                            }
                            user.update(&users).await;

                            message.messages.push(Message{
                                role: MessageRole::Tool,
                                content: serde_json::to_string(&user.trade).unwrap(),
                            });

                        }
                        _ => {
                            message.messages.push(Message{
                                role: MessageRole::Tool,
                                content: "Error gettingt the requested information".to_string(),
                            });
                        }
                    }
                }

                a = client.post("http://localhost:11434/api/chat")
                    .json(&message)
                    .send()
                    .await
                    .unwrap()
                    .json::<ChatMessageResponse>()
                    .await
                    .unwrap();

                info!("{:?}", message);
                info!("{:?}", a);
                s.send(InternalCommand::DslResponse(serde_json::to_string(&a.message.tool_calls).unwrap())).unwrap()
            }
            _ => {}
        }
    }
}