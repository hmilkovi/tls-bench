use clap::Parser;

use std::{io, net::SocketAddr, sync::Arc, thread::available_parallelism};
use tokio::{net, sync::mpsc, task};
use tokio_util::sync::CancellationToken;

mod cli;
mod controller;
mod math;
mod tls;

/// Simple program to greet a person
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Endpoint to run TLS benchmark against
    #[arg(short, long)]
    endpoint: String,

    /// Protocol to use when running TLS benchmark
    #[arg(short, value_enum, default_value_t = Protocol::Tcp)]
    protocol: Protocol,

    /// TLS version number
    #[arg(short, value_enum, default_value_t = TlsVersion::Tls12)]
    tls_version: TlsVersion,

    /// TLS Zero RTT boolean
    #[arg(short, long, default_value_t = false)]
    zero_rtt: bool,

    /// Duration of benchamrk test in seconds
    #[arg(short, long, default_value_t = 0)]
    duration: u64,

    /// Max concurrently running workers, defaults to available_parallelism
    #[arg(short, long, default_value_t = available_parallelism().unwrap().get())]
    concurrently: usize,

    /// Timeout of tcp connection & tls handshake in miliseconds
    #[arg(long, default_value_t = 500)]
    timeout_ms: u64,

    /// Maximum TLS handshakes per seconds
    #[arg(short, long, default_value_t = 1000)]
    max_handshakes_per_second: u64,

    /// Ramp up seconds, eatch step up per second is calculated = max_handshakes_per_second * elapsed_seconds / ramp_up_sec
    #[arg(short, long, default_value_t = 0)]
    ramp_up_sec: u64,
}

#[derive(clap::ValueEnum, Clone)]
enum Protocol {
    Tcp,
    Smtp,
}

#[derive(clap::ValueEnum, Clone)]
enum TlsVersion {
    Tls12,
    Tls13,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let mut tls_config = tls::tls_config(Some(cli.zero_rtt), Some(&[&rustls::version::TLS12]));
    if let TlsVersion::Tls13 = cli.tls_version {
        tls_config = tls::tls_config(Some(cli.zero_rtt), Some(&[&rustls::version::TLS13]));
    }

    let mut is_smtp = false;
    if let Protocol::Smtp = cli.protocol {
        is_smtp = true;
    }

    let endpoint: SocketAddr = net::lookup_host(cli.endpoint).await?.next().unwrap();

    let (tx, rx) = mpsc::unbounded_channel::<Result<tls::TlsDuration, std::io::Error>>();
    let token = CancellationToken::new();
    let cancel_token = token.clone();
    let mut tasks = task::JoinSet::new();
    tasks.spawn_blocking(move || {
        cli::show_progress_and_stats(
            cli.duration,
            cli.ramp_up_sec,
            cli.concurrently,
            rx,
            cancel_token,
        )
    });

    let traffic_controller = Arc::new(
        controller::TrafficController::new(cli.max_handshakes_per_second.try_into().unwrap()).await,
    );

    for _ in 0..cli.concurrently {
        let local_tls_config = tls_config.clone();
        let local_token = token.clone();
        let tx_result = tx.clone();
        let traffic_controller = traffic_controller.clone();
        tasks.spawn(async move {
            loop {
                tokio::select! {
                    _ = local_token.cancelled() => {
                        break;
                    },
                    _ = traffic_controller.acquire() => {
                        tls::tls_handshaker(endpoint, cli.timeout_ms, is_smtp, local_tls_config.clone(), tx_result.clone()).await;
                    }
                }
            }
        });
    }

    tasks.spawn(async move {
        traffic_controller.flow(cli.ramp_up_sec, token).await;
    });

    tasks.join_all().await;

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
