use std::{convert::TryInto, env, io};

use clap::{crate_description, crate_name, crate_version, App, Arg};
use futures_util::pin_mut;
use mqtt_broker::*;
use mqtt_opa::MakeOpaAuthorizer;
use tokio::time::{Duration, Instant};
use tracing::{info, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};

use mqttd::{shutdown, snapshot, Terminate};

#[tokio::main]
async fn main() -> Result<(), Terminate> {
    let subscriber = fmt::Subscriber::builder()
        .with_ansi(atty::is(atty::Stream::Stderr))
        .with_max_level(Level::TRACE)
        .with_writer(io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    run().await?;
    Ok(())
}

async fn run() -> Result<(), Error> {
    let config = create_app()
        .get_matches()
        .value_of("config")
        .map_or(BrokerConfig::new(), BrokerConfig::from_file)
        .map_err(InitializeBrokerError::LoadConfiguration)?;

    // Setup the shutdown handle
    let shutdown = shutdown::shutdown();
    pin_mut!(shutdown);

    let wasm_bytes = opa_go::wasm::compile("data.mqtt.allow", "policy.rego").unwrap();
    let authorizer = MakeOpaAuthorizer::from_bytes(wasm_bytes).unwrap();

    // Setup the snapshotter
    let mut persistor = FilePersistor::new(
        env::current_dir().expect("can't get cwd").join("state"),
        ConsolidatedStateFormat::default(),
    );
    info!("Loading state...");
    let state = persistor.load()?.unwrap_or_else(BrokerState::default);
    let broker = BrokerBuilder::default()
        .authenticator(|_| Ok(Some(AuthId::Anonymous)))
        .authorizer(authorizer)
        .state(state)
        .build();
    info!("state loaded.");

    let snapshotter = Snapshotter::new(persistor);
    let snapshot_handle = snapshotter.snapshot_handle();
    let mut shutdown_handle = snapshotter.shutdown_handle();
    let join_handle = tokio::spawn(snapshotter.run());

    // Tick the snapshotter
    let tick = tick_snapshot(
        Duration::from_secs(5 * 60),
        broker.handle(),
        snapshot_handle.clone(),
    );
    tokio::spawn(tick);

    // Signal the snapshotter
    let snapshot = snapshot::snapshot(broker.handle(), snapshot_handle.clone());
    tokio::spawn(snapshot);

    // Create configured transports
    let transports = config
        .transports()
        .iter()
        .map(|transport| transport.clone().try_into())
        .collect::<Result<Vec<_>, _>>()?;

    info!("Starting server...");
    let state = Server::from_broker(broker)
        .serve(transports, shutdown)
        .await?;

    // Stop snapshotting
    shutdown_handle.try_shutdown()?;
    let mut persistor = join_handle.await??;
    info!("state snapshotter shutdown.");

    info!("persisting state before exiting...");
    persistor.store(state)?;
    info!("state persisted.");
    info!("exiting... goodbye");

    Ok(())
}

async fn tick_snapshot(
    period: Duration,
    mut broker_handle: BrokerHandle,
    snapshot_handle: StateSnapshotHandle,
) {
    info!("Persisting state every {:?}", period);
    let start = Instant::now() + period;
    let mut interval = tokio::time::interval_at(start, period);
    loop {
        interval.tick().await;
        if let Err(e) = broker_handle.try_send(Message::System(SystemEvent::StateSnapshot(
            snapshot_handle.clone(),
        ))) {
            warn!(message = "failed to tick the snapshotter", error=%e);
        }
    }
}

fn create_app() -> App<'static, 'static> {
    App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
}
