use std::collections::HashMap;
use async_openai::types::{ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequestArgs, FunctionObject, FunctionObjectArgs};
use tokio::sync::mpsc::Receiver;
use chrono::Utc;
use log::{error, info};
use ollama_rs::{IntoUrlSealed, Ollama};
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use schemars::{schema_for, Schema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_sdk::signer::Signer;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{mongo, pg_conn};
use crate::prompt_type::{BuyPrompt, SellPrompt};
use crate::rpc::{rpc1, rpc1_unchecked};
use crate::trade_type::{Trade, TradeRequest2};
use crate::type_dsl::*;
use crate::app_user_type::UserWithId;

pub fn export_prompt_schema(){
    let schema = schema_for!(BuyPrompt);
    let schema = serde_json::to_string(&schema).unwrap();
    std::fs::write("prompts/buy.json",schema).unwrap();
    let schema = schema_for!(SellPrompt);
    let schema = serde_json::to_string(&schema).unwrap();
    std::fs::write("prompts/sell.json",schema).unwrap();
}

fn get_dsl_fns() -> Vec<(String,String, Schema)> {
    vec![
        ("Buy".to_string(),"Initiate a trade to buy at current prices".to_string(),schema_for!(Trade)),
        ("Sell".to_string(),"Initiate a trade to close a previous trade at current prices".to_string(),schema_for!(SellPrompt)),
    ].iter().map(|x| (x.0.clone(),x.1.clone(),x.2.clone())).collect::<Vec<_>>()
}
//
// fn get_all_tools() -> Vec<ToolCall> {
//     get_dsl_fns().into_iter().map(|(name,x)|{
// info!("what is schema {:?}",x);
//         ToolCall{
//             function: ToolCallFunction{
//                 name: name.to_string(),
//                 flattened_value: x.to_value()
//             },
//             kind: "function".to_string(),
//         }
//     }).collect::<Vec<_>>()
// }

fn get_open_ai_tools() -> Vec<ChatCompletionTool> {
    get_dsl_fns().into_iter().map(|(name,desc,schema)| {
        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name(name)
                    .description(desc)
                    .parameters(schema.to_value())
                    .build()
                    .unwrap()
            )
            .build()
            .unwrap()
    }).collect::<Vec<_>>()
}


pub async fn dsl_chan(chan:Chan,mut rec: Receiver<InternalCommand>) {
    let client = Client::new();
    let client = async_openai::Client::new();

    let pg = pg_conn().await;
    let mon = mongo().await;
    let users = mon.collection::<UserWithId>("users");
    while let Some(message) = rec.recv().await {
        match message {
            InternalCommand::Dsl(id,dsl,s) => {
info!("boom {:?}",schema_for!(TradeRequest2));
                let request = CreateChatCompletionRequestArgs::default()
                    .model("gpt-4-1106-preview")
                    .messages(vec![
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(dsl)
                            .build()
                            .unwrap()
                            .into()
                        ]
                    ).tools(get_open_ai_tools()).build().unwrap();


                let response_message = client
                    .chat()
                    .create(request)
                    .await
                    .unwrap()
                    .choices
                    .first()
                    .unwrap()
                    .message
                    .clone();



                if let Some(tool_calls) = response_message.tool_calls {
                    // let mut responses = vec![];
                    info!("{:?}",tool_calls);

                    for x in tool_calls {
                        let trade = serde_json::from_str::<Trade>(x.function.arguments.as_str()).unwrap();



                        // info!("{:?}",data);
                        // let sk = id.clone();
                        // let mut user = User::get_by_sk(sk.as_str(),&users).await.unwrap();
                        // user.trade.cfg.tp = Decimal::from_f64(data.tp).unwrap();
                        // user.trade.cfg.sl = Decimal::from_f64(data.sl).unwrap();
                        // user.update(&users).await;
                        //
                        // responses.push(json!({
                        //     "function_name":"Config",
                        //     "message":"User configuration updated",
                        // }));
                    }
                }
                else {
                    info!("{:?}", response_message);
                }



                // info!("{:?}", response_message);

                // let mut a = client.post("http://localhost:11434/api/chat")
                //     .json(&message)
                //     .send()
                //     .await
                //     .unwrap()
                //     .text()
                //     .await
                //     .unwrap();
                //
                // let r = serde_json::from_str::<ChatMessageResponse>(&a);
                // if r.is_err() {
                //     error!("{a} {:?}",r);
                //
                //     // message.messages.push(
                //     //     Message{
                //     //         role: MessageRole::Assistant,
                //     //         content: format!("It seems there was an error with previous response - {}",r.unwrap_err()),
                //     //     }
                //     // );
                //     // message.messages.push(
                //     //     Message{
                //     //         role: MessageRole::Tool,
                //     //         content:a,
                //     //     }
                //     // );
                //     //
                //     // a = client.post("http://localhost:11434/api/chat")
                //     //     .json(&message)
                //     //     .send()
                //     //     .await
                //     //     .unwrap()
                //     //     .text()
                //     //     .await
                //     //     .unwrap();
                //     info!("{:?}", a);
                // }


                // for x in a.message.tool_calls {
                //     match x.function {
                //         PromptResponse::Config(data) => {
                //             let sk = id.clone();
                //             let mut user = User::get_by_sk(sk.as_str(),&users).await.unwrap();
                //             user.trade.cfg.tp = Decimal::from_f64(data.tp).unwrap();
                //             user.trade.cfg.sl = Decimal::from_f64(data.sl).unwrap();
                //             user.update(&users).await;
                //
                //             message.messages.push(Message{
                //                 role: MessageRole::Tool,
                //                 content: serde_json::to_string(&user.trade).unwrap(),
                //             });
                //
                //         }
                //         _ => {
                //             message.messages.push(Message{
                //                 role: MessageRole::Tool,
                //                 content: "Error getting the requested information".to_string(),
                //             });
                //         }
                //     }
                // }
                //

                //
                // info!("{:?}", a);
                // s.send(InternalCommand::DslResponse(serde_json::to_string(&a.message.tool_calls).unwrap())).unwrap()
            }
            _ => {}
        }
    }
}