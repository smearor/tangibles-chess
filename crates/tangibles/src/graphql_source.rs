use futures::SinkExt;
use futures::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tracing::debug;
use tracing::warn;

use crate::DaemonError;
use crate::DeviceInfo;
use crate::FieldEvent;
use crate::FieldSetSpec;
use crate::TangibleSource;
use crate::TouchPoint;

/// GraphQL client of the `tangible-objects` daemon.
///
/// Queries and mutations go over `POST {endpoint}/graphql`; the
/// `fieldEvents` subscription runs on a `graphql-transport-ws` WebSocket
/// at `GET {endpoint}/graphql`, decoded into [`FieldEvent`]s and pushed
/// into the `events()` channel.
///
/// `connect` spawns the WebSocket task on the calling Tokio runtime -
/// the GTK app hosts a dedicated runtime thread for it.
pub struct GraphQlSource {
    /// Shared HTTP client for POST operations.
    http: reqwest::Client,
    /// Endpoint URL including `/graphql`.
    url: String,
    /// Live field-event stream.
    receiver: async_channel::Receiver<FieldEvent>,
    /// The WebSocket reader task; kept alive by holding the handle.
    _task: tokio::task::JoinHandle<()>,
}

/// The `fieldEvents` subscription document.
const FIELD_EVENTS_SUB: &str = r#"subscription FieldEvents($set: String) {
    fieldEvents(set: $set) { kind fieldSet field count skipped }
}"#;

impl GraphQlSource {
    /// Connects to the daemon at `endpoint` (e.g. `http://127.0.0.1:8080`,
    /// with or without `/graphql`) and starts the `fieldEvents`
    /// subscription for the chess field set.
    pub async fn connect(endpoint: &str) -> Result<Self, DaemonError> {
        let url = if endpoint.ends_with("/graphql") {
            endpoint.to_string()
        } else {
            format!("{}/graphql", endpoint.trim_end_matches('/'))
        };
        let ws_url = url.replace("https://", "wss://").replace("http://", "ws://");
        let (tx, receiver) = async_channel::bounded(256);
        let mut request = ws_url.into_client_request().map_err(|e| DaemonError::Transport(e.to_string()))?;
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("graphql-transport-ws"));
        let (mut socket, _response) = connect_async(request)
            .await
            .map_err(|e| DaemonError::Unavailable(format!("websocket connect failed: {e}")))?;
        Self::ws_send(&mut socket, &serde_json::json!({"type": "connection_init"})).await?;
        Self::ws_send(
            &mut socket,
            &serde_json::json!({
                "id": "fields",
                "type": "subscribe",
                "payload": {
                    "query": FIELD_EVENTS_SUB,
                    "variables": { "set": crate::FIELD_SET_NAME },
                },
            }),
        )
        .await?;
        let task = tokio::spawn(async move {
            while let Some(message) = socket.next().await {
                let Ok(message) = message else {
                    break;
                };
                let Message::Text(text) = message else {
                    continue;
                };
                let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                match frame.get("type").and_then(|t| t.as_str()) {
                    Some("next") => {
                        if let Some(data) = frame.pointer("/payload/data/fieldEvents") {
                            match serde_json::from_value::<FieldEvent>(data.clone()) {
                                Ok(event) => {
                                    if tx.try_send(event).is_err() {
                                        warn!("field event channel full/closed - dropping event");
                                    }
                                }
                                Err(err) => warn!("undecodable field event: {err}"),
                            }
                        }
                    }
                    Some("ping") => {
                        let _ = socket.send(Message::Text("{\"type\":\"pong\"}".into())).await;
                    }
                    Some("error") => {
                        warn!("subscription error: {}", frame.get("payload").cloned().unwrap_or_default());
                    }
                    Some("complete") | Some("connection_ack") => {}
                    Some("pong") => {}
                    other => debug!("unhandled ws frame: {other:?}"),
                }
            }
            debug!("field event subscription ended");
        });
        Ok(Self {
            http: reqwest::Client::new(),
            url,
            receiver,
            _task: task,
        })
    }

    /// Writes a JSON frame to the websocket.
    async fn ws_send(
        socket: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
        value: &serde_json::Value,
    ) -> Result<(), DaemonError> {
        socket
            .send(Message::Text(value.to_string().into()))
            .await
            .map_err(|e| DaemonError::Transport(e.to_string()))
    }

    /// Runs a GraphQL operation and extracts `data`.
    async fn gql(&self, query: &str, variables: serde_json::Value) -> Result<serde_json::Value, DaemonError> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let response: serde_json::Value = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DaemonError::Transport(e.to_string()))?
            .json()
            .await
            .map_err(|e| DaemonError::Decode(e.to_string()))?;
        if let Some(errors) = response.get("errors") {
            return Err(DaemonError::GraphQl(errors.to_string()));
        }
        response
            .get("data")
            .cloned()
            .ok_or_else(|| DaemonError::Decode("response lacks `data`".to_string()))
    }
}

impl TangibleSource for GraphQlSource {
    fn events(&self) -> async_channel::Receiver<FieldEvent> {
        self.receiver.clone()
    }

    async fn device_info(&self) -> Result<DeviceInfo, DaemonError> {
        let data = self
            .gql("query { deviceInfo { name xRes yRes xMin xMax yMin yMax } }", serde_json::json!({}))
            .await?;
        serde_json::from_value(data["deviceInfo"].clone()).map_err(|e| DaemonError::Decode(e.to_string()))
    }

    async fn touch_points(&self) -> Result<Vec<TouchPoint>, DaemonError> {
        let data = self.gql("query { touchPoints { slot trackingId pos { x y } } }", serde_json::json!({})).await?;
        serde_json::from_value(data["touchPoints"].clone()).map_err(|e| DaemonError::Decode(e.to_string()))
    }

    async fn register_field_set(&self, spec: &FieldSetSpec) -> Result<(), DaemonError> {
        self.gql(
            "mutation RegisterFieldSet($input: RegisterFieldSetInput!, $owner: String) {
                registerFieldSet(input: $input, owner: $owner)
            }",
            serde_json::json!({ "input": spec, "owner": crate::REGISTRATION_OWNER }),
        )
        .await?;
        Ok(())
    }

    async fn unregister(&self) -> Result<(), DaemonError> {
        self.gql("mutation { unregisterOwner(owner: \"tangibles-chess\") }", serde_json::json!({}))
            .await?;
        Ok(())
    }
}
