mod config;

use std::time::Duration;

use backoff::ExponentialBackoff;
use dcs_grpc_server::rpc::dcs::mission_client::MissionClient;
use dcs_grpc_server::rpc::dcs::{Event, StreamEventsRequest};
use futures_util::future::{select, FutureExt};
use tonic::{transport, Request, Status};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "event_logger=trace".to_owned());
    let registry = tracing_subscriber::registry().with(
        tracing_subscriber::filter::EnvFilter::new(filter)
            .and_then(tracing_subscriber::fmt::layer()),
    );
    registry.init();

    let backoff = ExponentialBackoff {
        // never wait longer than 30s for a retry
        max_interval: Duration::from_secs(30),
        // never stop trying
        max_elapsed_time: None,
        ..Default::default()
    };

    select(
        Box::pin(backoff::future::retry_notify(
            backoff,
            || async { run().await.map_err(backoff::Error::Transient) },
            |err, backoff: Duration| {
                tracing::error!(
                    %err,
                    backoff = %format!("{:.2}s", backoff.as_secs_f64()),
                    "retrying after error"
                );
            },
        )),
        Box::pin(tokio::signal::ctrl_c().map(|_| ())),
    )
    .await;
}

async fn run() -> Result<(), Error> {
    let addr = "http://127.0.0.1:50051"; // TODO: move to config
    tracing::debug!(endpoint = addr, "Connecting to gRPC server");
    let endpoint = transport::Endpoint::from_static(addr).keep_alive_while_idle(true);
    let mut client = MissionClient::connect(endpoint).await?;
    let mut events = client
        .stream_events(Request::new(StreamEventsRequest {}))
        .await?
        .into_inner();

    loop {
        tokio::select! {
            event = events.message() => handle_event(event?).await?,
        }
    }
}

async fn handle_event(event: Option<Event>) -> Result<(), Error> {
    let event = event.ok_or(Error::End)?;

    tracing::debug!(?event, "received event");

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Grpc(#[from] Status),
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error("event stream ended")]
    End,
}
