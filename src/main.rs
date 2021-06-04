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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::debug!("debug mode");

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
                            // Slack Reminder
                            if msg.user == "USLACKBOT" && msg.text.contains("logger random") {
                                loggger_random(&cfg, &msg).await;
                            }

                            log::debug!("msg: \"{}\"", &msg.text);
                            match msg.text.as_str() {
                                "logger random" => loggger_random(&cfg, &msg).await,
                                "logger list" => {
                                    let list = &cfg.member;
                                    let list: String = list
                                        .iter()
                                        .map(|m| {
                                            let mut m = m.clone();
                                            m += "\n";
                                            m
                                        })
                                        .collect();
                                    slack::post_message(&cfg.bot_token, &cfg.channel, &list).await
                                }
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

    slack::post_message(&cfg.bot_token, &cfg.channel, logger).await;
}
