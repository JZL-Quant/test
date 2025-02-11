// cargo.toml依赖
// 需要连接香港ip地址，否则无法连接
/*
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.18", features = ["native-tls"] }
serde_json = "1.0"
futures-util = "0.3"
*/

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::{json, Value};
use tokio::signal;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

#[tokio::main]
async fn main() {
    // OKX WebSocket API 地址（公共频道）
    let url = "wss://wspap.okx.com:8443/ws/v5/public";

    // 建立 WebSocket 连接
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect to OKX");

    println!("Connected to OKX WebSocket!");

    // 订阅 BTC-USDT 的 ticker 数据
    let subscribe_msg = json!({
        "op": "subscribe",
        "args": [
            {
                "channel": "tickers",
                "instId": "BTC-USDT"
            }
        ]
    });

    // 发送订阅消息
    ws_stream.send(Message::Text(subscribe_msg.to_string())).await.expect("Failed to send subscribe message");
    println!("Subscribed to BTC-USDT ticker!");

    // 监听 WebSocket 消息
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("Received Ctrl+C, closing connection...");
            // 使用 CloseCode::Iana 作为关闭代码
            ws_stream.close(Some(CloseFrame { code: CloseCode::Iana(1000), reason: "".into() })).await.expect("Failed to close WebSocket");
        }
        _ = async {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // 解析并提取 BidPrice 和 AskPrice
                        if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                            if let Some(data) = parsed["data"].as_array() {
                                for item in data {
                                    let bid_price = item["bidPx"].as_str().unwrap_or("N/A");
                                    let ask_price = item["askPx"].as_str().unwrap_or("N/A");
                                    println!("BidPrice: {}, AskPrice: {}", bid_price, ask_price);
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        // 处理 Ping 消息，回复 Pong
                        ws_stream.send(Message::Pong(vec![])).await.expect("Failed to send Pong");
                    }
                    Ok(other) => {
                        println!("Other message: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                }
            }
        } => {}
    }
}


// 运行代码
fn hello(){
    println!("Hello, world!");
}