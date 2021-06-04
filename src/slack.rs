use std::collections::HashMap;

#[derive(serde::Deserialize, Debug)]
pub struct WsUrlResponseJson {
    pub ok: bool,
    pub url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum WsUrlResponseError {
    NoUrl,
    Error(String),
    Surf(surf::Error),
    UrlParse(url::ParseError),
    Unknown,
}

#[derive(Debug)]
pub struct WsUrlResult(Result<url::Url, WsUrlResponseError>);

impl WsUrlResult {
    pub fn unwrap(self) -> url::Url {
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

#[derive(serde::Serialize, Debug)]
pub struct MessageAck<'a> {
    envelope_id: &'a str,
    payload: Option<&'a str>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Message<'a> {
    Hello {
        num_connections: u8,
        debug_info: Option<DebugInfo<'a>>,
        connection_info: ConnectionInfo<'a>,
    },
    Disconnect {
        reason: &'a str,
    },
    EventsApi(Box<EventsApiMessage<'a>>),
}

#[derive(serde::Deserialize, Debug)]
pub struct EventsApiMessage<'a> {
    pub envelope_id: &'a str,
    pub payload: EventsApiPayload<'a>,
    pub accepts_response_payload: bool,
    pub retry_attempt: usize,
    pub retry_reason: &'a str,
}

impl<'a> EventsApiMessage<'a> {
    pub fn ack(&self, payload: Option<&'a str>) -> MessageAck<'a> {
        MessageAck {
            envelope_id: &self.envelope_id,
            payload,
        }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct DebugInfo<'a> {
    host: &'a str,
    build_number: u32,
    approximate_connection_time: usize,
}
#[derive(serde::Deserialize, Debug)]
pub struct ConnectionInfo<'a> {
    app_id: &'a str,
}

#[derive(serde::Deserialize, Debug)]
pub struct EventsApiPayload<'a> {
    token: &'a str,
    team_id: &'a str,
    api_app_id: &'a str,
    pub event: Event<'a>,
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
pub enum Event<'a> {
    Message(EventMessage<'a>),
    AppMention {},
    _Dummy { hoge: &'a str }, // これがないとserde::Deserializeのマクロ展開でライフタイムがいいかんじにならず死ぬ
}

#[derive(serde::Deserialize, Debug)]
pub struct EventMessage<'a> {
    client_msg_id: Option<&'a str>,
    bot_id: Option<&'a str>,
    pub text: String,
    pub user: &'a str,
    ts: &'a str,
    team: Option<&'a str>,
    // blocks
    pub channel: &'a str,
    event_ts: &'a str,
    pub channel_type: &'a str,
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
pub enum ParseMessageError {
    JsonParse(serde_json::Error),
    _Unknown,
}

#[derive(Debug)]
//#[serde(tag = "ok")]
pub enum PostInfoRaw<'a> {
    //#[serde(rename = "true")]
    Ok(PostInfo<'a>),

    //#[serde(rename = "false")]
    Error(PostError),
    //#[serde(rename = "_dummy")]
    //_Dummy { hoge: &'a str },
}

// https://stackoverflow.com/questions/65575385/deserialization-of-json-with-serde-by-a-numerical-value-as-type-identifier/65576570#65576570
impl<'de: 'a, 'a> serde::Deserialize<'de> for PostInfoRaw<'a> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<PostInfoRaw<'a>, D::Error> {
        use serde_json::Value;
        let v = Value::deserialize(d)?;

        match v.get("ok").and_then(Value::as_bool).unwrap() {
            true => {
                let pi = PostInfo::deserialize(v);
                if let Ok(r) = pi {
                    Ok(PostInfoRaw::Ok(r))
                } else {
                    Err(serde::de::Error::unknown_field("PostInfo", &["?"]))
                }
            }
            false => Ok(PostInfoRaw::Error(PostError::deserialize(v).unwrap())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::slack::*;

    #[test]
    fn deserialize_eventmessage() {
        let ems = r#"{
          "bot_id": "B01",
          "text": "\u30ea\u30de\u30a4\u30f3\u30c0\u30fc : logger random.",
          "user": "USLACKBOT",
          "ts": "1622817919.003000",
          "team": "T01U5SDH0QH",
          "channel": "C02389A6YGJ",
          "event_ts": "1622817919.003000",
          "channel_type": "channel"
        }"#;
        let _em: EventMessage = serde_json::from_str(&ems).unwrap();
    }

    #[test]
    fn deserialize_postinfo_error() {
        let es = "{\"ok\":false,\"error\":\"channel_not_found\"}";
        let _e: PostInfoRaw = serde_json::from_str(&es).unwrap();

        let es = "{\"ok\":false,\"error\":\"not_in_channel\"}";
        let _e: PostInfoRaw = serde_json::from_str(&es).unwrap();
    }

    #[test]
    fn deserialize_postinfo_ok() {
        let os = "{\"ok\":true,\"channel\":\"C02389A6YGJ\",\"ts\":\"1622717214.001900\",\"message\":{\"bot_id\":\"hoge\",\"type\":\"message\",\"text\":\"sksat\",\"user\":\"U01UR56PLLC\",\"ts\":\"1622717214.001900\",\"team\":\"T01U5SDH0QH\",\"bot_profile\":{\"id\":\"B01UK6J77JP\",\"deleted\":false,\"name\":\"archbot\",\"updated\":1618578906,\"app_id\":\"A01UC646PGW\",\"icons\":{\"image_36\":\"https:\\/\\/a.slack-edge.com\\/80588\\/img\\/plugins\\/app\\/bot_36.png\",\"image_48\":\"https:\\/\\/a.slack-edge.com\\/80588\\/img\\/plugins\\/app\\/bot_48.png\",\"image_72\":\"https:\\/\\/a.slack-edge.com\\/80588\\/img\\/plugins\\/app\\/service_72.png\"},\"team_id\":\"T01U5SDH0QH\"}}}";
        let _o: PostInfoRaw = serde_json::from_str(&os).unwrap();
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct PostInfo<'a> {
    channel: &'a str,
    ts: &'a str,
    message: EventMessage<'a>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "error")]
pub enum PostError {
    //pub error: String,
    ChannelNotFound,
    NotInChannel,

    #[serde(other)]
    Unknown,
}

pub async fn get_ws_url(token: &str) -> WsUrlResult {
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

pub fn parse_message(json: &str) -> Result<Message, ParseMessageError> {
    let json_pretty = jsonxf::pretty_print(json).unwrap();
    log::debug!("pretty: {}", json_pretty);
    let msg: Message = serde_json::from_str(&json)?;
    log::debug!("parsed msg: {:?}", msg);
    Ok(msg)
}

impl From<serde_json::Error> for ParseMessageError {
    fn from(e: serde_json::Error) -> ParseMessageError {
        ParseMessageError::JsonParse(e)
    }
}

pub async fn post_message(token: &str, channel: &String, msg: &String) {
    let mut q = HashMap::new();
    q.insert("channel", &channel);
    q.insert("text", &msg);
    let url = url::Url::parse_with_params("https://slack.com/api/chat.postMessage", &q).unwrap();

    let r = surf::post(url)
        .header(
            surf::http::headers::AUTHORIZATION,
            format!("Bearer {}", &token),
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
    let info: Result<PostInfoRaw, _> = serde_json::from_str(&r);
    if let Ok(i) = info {
        log::info!("{:?}", i)
    } else {
        log::error!("{:?}", info.err().unwrap());
    }
}
