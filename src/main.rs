use std::collections::HashMap;
use std::fs;
use std::io::Read;

use async_std::stream::StreamExt;
use futures_util::sink::SinkExt;

use rand::seq::SliceRandom;

mod slack;

#[derive(Debug, serde::Deserialize)]
struct Config {
    app_token: String,
    bot_token: String,
    channel: String,
    member: Vec<String>,
}

#[derive(serde::Serialize, Debug)]
struct PostMessage<'a> {
    token: &'a str,
    channel: &'a str,
    text: &'a str,
    username: &'a str,
}

async fn auth_test(token: &str) {
    let r = surf::post("https://slack.com/api/auth.test")
        .header(
            surf::http::headers::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .recv_string()
        .await
        .unwrap();
    dbg!(r);
}

#[async_std::main]
async fn main() {
    let mut cfg_file = fs::File::open("config.toml").unwrap();
    let mut cfg = String::new();
    let _ = cfg_file.read_to_string(&mut cfg);
    let cfg: Config = toml::from_str(&cfg).unwrap();

    auth_test(&cfg.bot_token).await;

    let res = slack::get_ws_url(&cfg.app_token).await;
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

                        let event = ea.payload.event;
                        match event {
                            slack::Event::Message(msg) => match msg.text {
                                "logger random" => {
                                    println!("logger random from {} by {}", msg.channel, msg.user);

                                    // choose
                                    let logger =
                                        &cfg.member.choose(&mut rand::thread_rng()).unwrap();

                                    let channel = &cfg.channel;
                                    let mut q = HashMap::new();
                                    q.insert("channel", &channel);
                                    q.insert("text", &logger);
                                    let url = url::Url::parse_with_params(
                                        "https://slack.com/api/chat.postMessage",
                                        &q,
                                    )
                                    .unwrap();

                                    let r = surf::post(url)
                                        .header(
                                            surf::http::headers::AUTHORIZATION,
                                            format!("Bearer {}", &cfg.bot_token),
                                        )
                                        .recv_string()
                                        .await
                                        .unwrap();
                                    dbg!(r);
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => eprintln!("Unknown message: {:?}", msg),
        }
    }
}
