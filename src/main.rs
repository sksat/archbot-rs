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
    log::debug!("{:?}", r);
}

#[async_std::main]
async fn main() {
    std::env::set_var("RUST_LOG", "archbot=info");
    env_logger::init();

    log::info!("loading config");
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

    log::debug!("{:?}", &stream);

    while let Some(msg) = stream.next().await {
        if let Err(me) = msg {
            // like Protocol(ResetWithoutClosingHandshake)
            log::error!("msg error: {:?}", me);
            continue;
        }
        let msg = msg.unwrap();
        match msg {
            tungstenite::Message::Ping(_) => {
                log::debug!("ping");
            }
            tungstenite::Message::Text(txt) => {
                log::debug!("msg: {}", txt);
                let msg = slack::parse_message(&txt);
                if msg.is_err() {
                    log::error!("{:?}", msg.err().unwrap());
                    continue;
                }
                let msg = msg.unwrap();
                match msg {
                    slack::Message::EventsApi(ea) => {
                        // reply ack
                        let ack = ea.ack(None);
                        let ack = serde_json::to_string(&ack).unwrap();
                        stream.send(tungstenite::Message::Text(ack)).await.unwrap();

                        let event = ea.payload.event;
                        if let slack::Event::Message(msg) = event {
                            match msg.text {
                                "logger random" => loggger_random(&cfg, &msg).await,
                                _ => {}
                            }
                        }
                    }
                    _ => log::error!("unknown message: {:?}", msg),
                }
            }
            _ => log::error!("Unknown message: {:?}", msg),
        }
    }
}

async fn loggger_random(cfg: &Config, msg: &slack::EventMessage<'_>) {
    log::info!("logger random from {} by {}", msg.channel, msg.user);

    // choose
    let logger = &cfg.member.choose(&mut rand::rngs::OsRng).unwrap();
    log::info!("logger choosed: {}", logger);

    let channel = &cfg.channel;
    let mut q = HashMap::new();
    q.insert("channel", &channel);
    q.insert("text", &logger);
    let url = url::Url::parse_with_params("https://slack.com/api/chat.postMessage", &q).unwrap();

    let r = surf::post(url)
        .header(
            surf::http::headers::AUTHORIZATION,
            format!("Bearer {}", &cfg.bot_token),
        )
        .recv_string()
        .await;

    if r.is_err() {
        log::error!("POST: {:?}", r.err().unwrap());
        return;
        //continue;
    }

    let r = r.unwrap();
    log::info!("{}", r);
    let info: Result<slack::PostInfoRaw, _> = serde_json::from_str(&r);
    if let Ok(i) = info {
        log::info!("{:?}", i)
    } else {
        log::error!("{:?}", info.err().unwrap());
    }
}
