use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{error, info};
use okx_api_rs::{futures::{TryStreamExt, SinkExt}, okx::{client::{Okx, Credentials}, model::websocket::{WSTopic, OkxWebsocketMsg}}};
use tokio::{time::sleep, sync::mpsc::{UnboundedSender, UnboundedReceiver}};

use crate::{model::{TraderDataWithTs, TraderData, Orderbook, OrderResp}, BOT, API_KEY, API_SECRET, API_PASSPHRASE};

pub async fn subscribe_ticker(coins: Vec<String>, trader_tx: UnboundedSender<TraderDataWithTs>) {
    loop {
        match Okx::new(None) {
            Ok(okx) => {
                let mut ws = okx.websocket();
                match ws.subscribe(vec![WSTopic::Ticker(coins.clone())], false, None).await {
                    Ok(_) => {
                        while let Ok(message) = ws.try_next().await {
                            if let Some(msg) = message {
                                match msg {
                                    OkxWebsocketMsg::Ticker(ticker) => {
                                        for data in ticker.data {
                                            // 合约
                                            if ticker.arg.inst_id.ends_with("-SWAP") {
                                                trader_tx.send(TraderDataWithTs { 
                                                    ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(),
                                                    data: TraderData::FUTUREORDERBOOK(
                                                        Orderbook {
                                                            best_ask: data.ask_px.parse::<f64>().unwrap(),
                                                            best_ask_size: data.ask_sz.parse::<f64>().unwrap(),
                                                            best_bid: data.bid_px.parse::<f64>().unwrap(),
                                                            best_bid_size: data.bid_sz.parse::<f64>().unwrap(),
                                                        }
                                                    )
                                                }).unwrap_or_else(|e| error!("{e}"));
                                            } else {
                                                trader_tx.send(TraderDataWithTs { 
                                                    ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(),
                                                    data: TraderData::SPOTORDERBOOK(
                                                        Orderbook {
                                                            best_ask: data.ask_px.parse::<f64>().unwrap(),
                                                            best_ask_size: data.ask_sz.parse::<f64>().unwrap(),
                                                            best_bid: data.bid_px.parse::<f64>().unwrap(),
                                                            best_bid_size: data.bid_sz.parse::<f64>().unwrap(),
                                                        }
                                                    )
                                                }).unwrap_or_else(|e| error!("{e}"));
                                            }
                                        }
                                    },
                                    _ => ()
                                }
                            } else {
                                BOT.warn("websockt received empty message").await;
                            }
                        }
                    },
                    Err(e) => {
                        BOT.warn(format!("websocket subscribe ticker failed, {}", e)).await;
                        sleep(Duration::from_secs(3)).await;
                    }
                }
            },
            Err(e) => {
                BOT.warn(format!("okx client network error: {}", e)).await;
                sleep(Duration::from_secs(3)).await;
            }
       }
    }
}

pub async fn order_websocket(coins: Vec<String>, trader_tx: UnboundedSender<TraderDataWithTs>, subscribe_rx: UnboundedReceiver<String>) {
    match Okx::new(Some(Credentials { 
        api_key: API_KEY.to_string(),
        secret_key: API_SECRET.to_string(),
        passphrase: API_PASSPHRASE.to_string()
    })) {
        Ok(okx) => {
            let mut ws = okx.websocket();
            let mut ws_topic = vec![];
            for coin in coins {
                ws_topic.push(WSTopic::SpotOrders(coin.to_owned()));
                ws_topic.push(WSTopic::MarginOrders(coin.to_owned()));
                ws_topic.push(WSTopic::SwapOrders(coin.to_owned()));
            }
            match ws.subscribe(ws_topic, true, Some(subscribe_rx)).await {
                Ok(_) => {
                    while let Ok(message) = ws.try_next().await {
                        if let Some(msg) = message {
                            match msg {
                                OkxWebsocketMsg::OrdersMsg(order) => {
                                    info!("order_websocket orders resp: {:?}", &order);
                                    if let Some(order_data) = order.data.get(0) {
                                        trader_tx.send(
                                            TraderDataWithTs {
                                                ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(),
                                                data: TraderData::ORDER_RESP(order),
                                            }
                                        ).unwrap_or_else(|e| error!("{e}"));
                                    }
                                },
                                OkxWebsocketMsg::BatchOrder(batchorder) => {
                                    info!("order_websocket batch order resp: {:?}", batchorder);
                                    trader_tx.send(
                                        TraderDataWithTs {
                                            ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(),
                                            data: TraderData::BATCHORDER_RESP(batchorder.clone()),
                                        }
                                    ).unwrap_or_else(|e| error!("{e}"));                                
                                    if "0" != batchorder.code {
                                        BOT.warn(format!("batch order error: {:?}", batchorder)).await;
                                    }
                                },
                                _ => ()
                            }
                        } else {
                            BOT.warn("websockt received empty message").await;
                        }
                    }
                },
                Err(e) => {
                    BOT.warn(format!("place order websocket failed, {}", e)).await;
                },
            }
        },
        Err(e) => {
            BOT.warn(format!("okx client network error: {}", e)).await;
        },
    }
}