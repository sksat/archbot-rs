use async_std::stream::StreamExt;
use futures_util::sink::SinkExt;

#[derive(serde::Deserialize, Debug)]
struct WsUrlResponseJson {
    pub ok: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
enum WsUrlResponseError {
    NoUrl,
    Error(String),
    Surf(surf::Error),
    UrlParse(url::ParseError),
    Unknown,
}

#[derive(Debug)]
struct WsUrlResult(Result<url::Url, WsUrlResponseError>);

impl WsUrlResult {
    fn unwrap(self) -> url::Url {
        self.0.unwrap()
    }
}

impl From<WsUrlResult> for Result<url::Url, WsUrlResponseError> {
    fn from(f: WsUrlResult) -> Self {
        f.0
    }
}

impl From<surf::Result<WsUrlResponseJson>> for WsUrlResult {
    fn from(f: surf::Result<WsUrlResponseJson>) -> Self {
        if f.is_err() {
            let e = f.err().unwrap();
            return WsUrlResult(Err(WsUrlResponseError::Surf(e)));
        }
        let f = f.unwrap();
        if !f.ok {
            if f.error.is_none() {
                return WsUrlResult(Err(WsUrlResponseError::Unknown));
            }
            let e = f.error.unwrap();
            return WsUrlResult(Err(WsUrlResponseError::Error(e)));
        }

        if f.url.is_none() {
            return WsUrlResult(Err(WsUrlResponseError::NoUrl));
        }

        let url = f.url.unwrap();
        let res = url::Url::parse(&url);
        if res.is_err() {
            let err = res.err().unwrap();
            return WsUrlResult(Err(WsUrlResponseError::UrlParse(err)));
        }

        let url = res.unwrap();
        WsUrlResult(Ok(url))
    }
}

async fn get_ws_url(token: &str) -> WsUrlResult {
    let res: surf::Result<WsUrlResponseJson> =
        surf::post("https://slack.com/api/apps.connections.open")
            .header(
                surf::http::headers::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .recv_json()
            .await;
    res.into()
}

#[derive(serde::Serialize, Debug)]
struct MessageAck<'a> {
    envelope_id: &'a str,
    payload: Option<&'a str>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Message<'a> {
    Hello {
        num_connections: u8,
        debug_info: Option<DebugInfo<'a>>,
        connection_info: ConnectionInfo<'a>,
    },
    Disconnect {
        reason: &'a str,
    },
    EventsApi(EventsApiMessage<'a>),
}

#[derive(serde::Deserialize, Debug)]
struct EventsApiMessage<'a> {
    envelope_id: &'a str,
    payload: EventsApiPayload<'a>,
    accepts_response_payload: bool,
    retry_attempt: usize,
    retry_reason: &'a str,
}

#[derive(serde::Deserialize, Debug)]
struct DebugInfo<'a> {
    host: &'a str,
    build_number: u32,
    approximate_connection_time: usize,
}
#[derive(serde::Deserialize, Debug)]
struct ConnectionInfo<'a> {
    app_id: &'a str,
}

#[derive(serde::Deserialize, Debug)]
struct EventsApiPayload<'a> {
    token: &'a str,
    team_id: &'a str,
    api_app_id: &'a str,
    event: Event<'a>,
    #[serde(rename = "type")]
    typ: &'a str,
    event_id: &'a str,
    event_time: usize,
    authorizations: Vec<Authorization<'a>>,
    is_ext_shared_channel: bool,
    event_context: &'a str,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Event<'a> {
    Message {
        client_msg_id: &'a str,
        text: &'a str,
        user: &'a str,
        ts: &'a str,
        team: &'a str,
        // blocks
        channel: &'a str,
        event_ts: &'a str,
        channel_type: &'a str,
    },
}

#[derive(serde::Deserialize, Debug)]
struct Authorization<'a> {
    //enterprise_id: Option<&'a str>,
    team_id: &'a str,
    user_id: &'a str,
    is_bot: bool,
    is_enterprise_install: bool,
}

#[derive(Debug)]
enum ParseMessageError {
    Unknown,
}

fn parse_message(json: &str) -> Result<Message, ParseMessageError> {
    let msg: Message = serde_json::from_str(json).unwrap();
    println!("{:?}", msg);
    Ok(msg)
}

#[async_std::main]
async fn main() {
    let token = "";
    let res = get_ws_url(&token).await;
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
                let msg = parse_message(&txt).unwrap();
                match msg {
                    Message::EventsApi(ea) => {
                        // reply ack
                        let envelope_id = ea.envelope_id;
                        stream
                            .send(tungstenite::Message::Text(
                                serde_json::to_string(&MessageAck {
                                    envelope_id,
                                    payload: None,
                                })
                                .unwrap(),
                            ))
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
            _ => eprintln!("Unknown message: {:?}", msg),
        }
    }
}
