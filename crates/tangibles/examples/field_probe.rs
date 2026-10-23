//! Live probe: subscribes to `fieldEvents` on the daemon and prints
//! every event for ~40 seconds - verifies that occupancy fields work
//! against the real panel.
//!
//! `cargo run -p tangibles-chess-tangibles --example field_probe -- <url>`

use futures_lite::StreamExt;
use miette::IntoDiagnostic;
use tangible_objects_client::Endpoint;
use tangible_objects_client::GraphQLClient;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let url = std::env::args().nth(1).unwrap_or_else(|| "http://192.168.178.81:8080".to_string());
    let endpoint = Endpoint::new(format!("{}/graphql", url.trim_end_matches('/')));
    let client = GraphQLClient::new(endpoint).into_diagnostic()?;

    println!("connected - subscribing to all field events (90s)");
    println!("place/lift an object on the panel now\n");

    let mut stream = Box::pin(client.field_events(None).await.into_diagnostic()?);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        if timeout.is_zero() {
            break;
        }
        match tokio::time::timeout(timeout, stream.next()).await {
            Ok(Some(Ok(event))) => {
                println!("{:?} set={:?} field={:?} count={:?}", event.kind, event.field_set, event.field, event.count);
            }
            Ok(Some(Err(err))) => println!("stream error: {err}"),
            Ok(None) => {
                println!("stream ended");
                break;
            }
            Err(_) => break,
        }
    }
    Ok(())
}
