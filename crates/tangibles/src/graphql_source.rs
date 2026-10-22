use futures_lite::StreamExt;
use tangible_objects_client::ClientError;
use tangible_objects_client::Endpoint;
use tangible_objects_client::GraphQLClient;
use tangible_objects_model::FieldSet;
use tangible_objects_model::FieldSetName;
use tangible_objects_model::Lifetime;
use tangible_objects_model::Owner;
use tracing::debug;
use tracing::warn;

use crate::DaemonError;
use crate::DeviceInfo;
use crate::FieldEvent;
use crate::TangibleSource;
use crate::TouchPoint;

/// Tangible source backed by the `tangible-objects` daemon.
///
/// Thin adapter over the upstream [`GraphQLClient`] (HTTP queries and
/// mutations, `graphql-transport-ws` subscriptions): the typed stream of
/// [`tangible_objects_model::TangibleEvent`]s is projected into our
/// narrower [`FieldEvent`] channel so the rest of the crate never sees
/// non-field payload.
///
/// `connect` spawns the subscription pump on the calling Tokio runtime -
/// the GTK app hosts a dedicated runtime thread for it.
pub struct GraphQlSource {
    /// The upstream typed client.
    client: GraphQLClient,
    /// Live field-event stream (our projection).
    receiver: async_channel::Receiver<FieldEvent>,
    /// The subscription pump task; kept alive by holding the handle.
    _task: tokio::task::JoinHandle<()>,
}

impl GraphQlSource {
    /// Connects to the daemon at `endpoint` (e.g. `http://127.0.0.1:8080`
    /// or the full `…/graphql` URL) and starts the `fieldEvents`
    /// subscription for the chess field set.
    pub async fn connect(endpoint: &str) -> Result<Self, DaemonError> {
        let endpoint = if endpoint.ends_with("/graphql") {
            Endpoint::new(endpoint)
        } else {
            Endpoint::new(format!("{}/graphql", endpoint.trim_end_matches('/')))
        };
        let client = GraphQLClient::new(endpoint)?;
        let set = FieldSetName::new(crate::FIELD_SET_NAME);
        let mut stream = Box::pin(client.field_events(Some(&set)).await?);
        let (tx, receiver) = async_channel::bounded(256);
        let task = tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(event) => {
                        if tx.try_send(FieldEvent::from(&event)).is_err() {
                            warn!("field event channel full/closed - dropping event");
                        }
                    }
                    Err(err) => {
                        warn!("field event stream error: {err} - asking caller to resync");
                        let _ = tx.try_send(FieldEvent::lagged(None));
                    }
                }
            }
            debug!("field event subscription ended");
        });
        Ok(Self { client, receiver, _task: task })
    }
}

impl TangibleSource for GraphQlSource {
    fn events(&self) -> async_channel::Receiver<FieldEvent> {
        self.receiver.clone()
    }

    async fn device_info(&self) -> Result<DeviceInfo, DaemonError> {
        self.client
            .device_info()
            .await?
            .map(DeviceInfo::from)
            .ok_or_else(|| DaemonError::Unavailable("daemon reports no device info".to_string()))
    }

    async fn touch_points(&self) -> Result<Vec<TouchPoint>, DaemonError> {
        Ok(self.client.touch_points().await?.iter().map(TouchPoint::from).collect())
    }

    async fn register_field_set(&self, spec: &FieldSet) -> Result<(), DaemonError> {
        self.client
            .register_field_set(spec, Some(&Owner::new(crate::REGISTRATION_OWNER)), Lifetime::Session)
            .await?;
        Ok(())
    }

    async fn unregister(&self) -> Result<(), DaemonError> {
        self.client.unregister_field_set(&FieldSetName::new(crate::FIELD_SET_NAME)).await?;
        Ok(())
    }
}

impl From<ClientError> for DaemonError {
    fn from(err: ClientError) -> Self {
        match err {
            ClientError::Http(_) | ClientError::Ws(_) | ClientError::NoData => DaemonError::Transport(err.to_string()),
            ClientError::Subscription(_) | ClientError::Protocol(_) | ClientError::FieldNotFound(_) => DaemonError::Unavailable(err.to_string()),
            ClientError::Graphql(value) => DaemonError::GraphQl(value.to_string()),
            ClientError::Decode(err) => DaemonError::Decode(err.to_string()),
        }
    }
}
