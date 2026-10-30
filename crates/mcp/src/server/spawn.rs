//! Spawning the Streamable-HTTP endpoint.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use rmcp::RoleServer;
use rmcp::model::ResourceUpdatedNotificationParam;
use rmcp::service::Peer;
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::StreamableHttpService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use tracing::error;
use tracing::info;

use super::chess_mcp::ChessMcp;
use crate::GameRequest;

/// Spawns the MCP Streamable-HTTP endpoint on a dedicated tokio runtime
/// thread; every tool call hops through `tx` onto the GTK main loop.
///
/// The default host allowlist is localhost-only - LAN deployments must
/// allow their own bound address.
pub fn spawn_http(
    tx: async_channel::Sender<GameRequest>,
    addr: SocketAddr,
    state_watch: tokio::sync::watch::Receiver<serde_json::Value>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(err) => {
                error!(%err, "MCP: failed to start tokio runtime");
                return;
            }
        };
        runtime.block_on(async move {
            // Host validation: a wildcard bind cannot predict which LAN
            // address clients connect through, so the allowlist is
            // disabled there. A concrete bind restricts the list to
            // loopback plus the bound address itself.
            let config = if addr.ip().is_unspecified() {
                StreamableHttpServerConfig::default().disable_allowed_hosts()
            } else {
                let allowed_hosts = [
                    "localhost".to_string(),
                    "127.0.0.1".to_string(),
                    "::1".to_string(),
                    addr.ip().to_string(),
                    addr.to_string(),
                ];
                StreamableHttpServerConfig::default().with_allowed_hosts(allowed_hosts)
            };
            let subscribers = Arc::new(Mutex::new(Vec::new()));
            // Forward every game-state change to subscribed peers as
            // `notifications/resources/updated`.
            tokio::spawn({
                let subscribers = subscribers.clone();
                let mut state_watch = state_watch;
                async move {
                    while state_watch.changed().await.is_ok() {
                        let peers: Vec<(String, Peer<RoleServer>)> = subscribers.lock().map(|subs| subs.clone()).unwrap_or_default();
                        for (uri, peer) in peers {
                            let _ = peer.notify_resource_updated(ResourceUpdatedNotificationParam::new(uri)).await;
                        }
                    }
                }
            });
            let service = StreamableHttpService::new(
                move || {
                    Ok(ChessMcp {
                        tx: tx.clone(),
                        subscribers: subscribers.clone(),
                    })
                },
                Arc::new(LocalSessionManager::default()),
                config,
            );
            let app = axum::Router::new().route_service("/mcp", service);
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    error!(%err, %addr, "MCP: bind failed");
                    return;
                }
            };
            info!(%addr, "MCP endpoint listening");
            if let Err(err) = axum::serve(listener, app).await {
                error!(%err, "MCP server failed");
            }
        });
    })
}
