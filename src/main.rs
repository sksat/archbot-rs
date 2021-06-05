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

    let start_msg = format!("archbot v{} started", env!("CARGO_PKG_VERSION"));
    log::info!("{}", &start_msg);
    log::debug!("debug mode");

    log::info!("loading config");
    let mut cfg_file = fs::File::open("config.toml").unwrap();
    let mut cfg = String::new();
    let _ = cfg_file.read_to_string(&mut cfg);
    let cfg: Config = toml::from_str(&cfg).unwrap();

    auth_test(&cfg.bot_token).await;

    slack::post_message(&cfg.bot_token, &cfg.channel, &start_msg).await;

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

    loop {
        let msg = stream.next().await;
        if msg.is_none() {
            continue;
        }
        let msg = msg.unwrap();
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
                                loggger_random(&cfg).await;
                            }

                            log::debug!("msg: \"{}\"", &msg.text);
                            if let Some(cmd) = msg.text.strip_prefix("logger ") {
                                log::info!("logger {} from {} by {}", &cmd, msg.channel, msg.user);
                                logger_cmd(&cfg, &cmd).await
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

async fn logger_cmd(cfg: &Config, cmd: &str) {
    let mut output = String::new();
    match cmd {
        "help" => {
            output += concat!(
                "usage: logger [COMMAND]\n",
                "commands:\n",
                "  `help`     Show this usage\n",
                "  `list`     Show `logger random` member\n",
                "  `random`   Choose logger\n"
            );
        }
        "list" => {
            let list = &cfg.member;
            output = list
                .iter()
                .map(|m| {
                    let mut m = m.clone();
                    m += "\n";
                    m
                })
                .collect();
        }
        "random" => loggger_random(&cfg).await,
        _ => {
            output += &format!(
                "[logger] '{}' is not a logger command.\nSee 'logger help'",
                &cmd
            );
        }
    }
    slack::post_message(&cfg.bot_token, &cfg.channel, &output).await
}

async fn loggger_random(cfg: &Config) {
    // choose
    let logger = &cfg.member.choose(&mut rand::rngs::OsRng).unwrap();
    log::info!("logger choosed: {}", logger);

    slack::post_message(&cfg.bot_token, &cfg.channel, logger).await;
}
