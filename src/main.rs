use std::fs;
use std::io::Read;

use async_std::stream::StreamExt;
use futures_util::sink::SinkExt;

mod slack;

#[derive(Debug, serde::Deserialize)]
struct Config {
    token: String,
}

#[async_std::main]
async fn main() {
    let mut cfg_file = fs::File::open("config.toml").unwrap();
    let mut cfg = String::new();
    let _ = cfg_file.read_to_string(&mut cfg);
    let cfg: Config = toml::from_str(&cfg).unwrap();

    let res = slack::get_ws_url(&cfg.token).await;
    let url: url::Url = res.unwrap();

    let domain = url.domain().unwrap();
    let stream_tcp = async_std::net::TcpStream::connect(format!("{}:443", domain))
        .await
        .unwrap();
    let stream_tls = async_tls::TlsConnector::default()
        .connect(domain, stream_tcp)
        .await
        .unwrap();
    let (mut stream, _) = async_tungstenite::client_async(url, stream_tls)
        .await
        .unwrap();

    dbg!(&stream);

    while let Some(msg) = stream.next().await {
        let msg = msg.unwrap();
        match msg {
            tungstenite::Message::Ping(_) => {
                println!("ping");
            }
            tungstenite::Message::Text(txt) => {
                println!("msg: {}", txt);
                let msg = slack::parse_message(&txt).unwrap();
                match msg {
                    slack::Message::EventsApi(ea) => {
                        // reply ack
                        let ack = ea.ack(None);
                        let ack = serde_json::to_string(&ack).unwrap();
                        stream.send(tungstenite::Message::Text(ack)).await.unwrap();
                    }
                    _ => {}
                }
            }
            _ => eprintln!("Unknown message: {:?}", msg),
        }
    }
}
