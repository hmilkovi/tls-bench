use clap::Parser;
use std::{io, net::SocketAddr, sync::Arc, thread::available_parallelism, time::Duration};
use tokio::{
    net,
    sync::Mutex,
    task::JoinSet,
    time::{interval, Instant},
};

mod tls;

/// Simple program to greet a person
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Endpoint to run TLS benchmark against
    #[arg(short, long)]
    endpoint: String,

    /// Protocol to use when runing TLS benchmark
    #[arg(short, value_enum, default_value_t = Protocol::Tcp)]
    protocol: Protocol,

    /// TLS version number, supported are v1.2 and v1.3
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
    #[arg(long, default_value_t = 1000)]
    timeout_ms: u64,

    /// Maximum TLS handshakes per seconds, defaults to zero which disables this throttle feature
    #[arg(short, long, default_value_t = 0)]
    max_handshakes_per_second: u64,
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
    match cli.tls_version {
        TlsVersion::Tls13 => {
            tls_config = tls::tls_config(Some(cli.zero_rtt), Some(&[&rustls::version::TLS13]));
        }
        _ => {}
    }

    let mut is_smtp = false;
    match cli.protocol {
        Protocol::Smtp => is_smtp = true,
        _ => {}
    }

    let endpoint: SocketAddr = net::lookup_host(cli.endpoint)
        .await?
        .into_iter()
        .nth(0)
        .unwrap();

    let mut tasks = JoinSet::new();

    let mut max_handshakes_per_second = 1;
    if cli.max_handshakes_per_second > 0 {
        max_handshakes_per_second = cli.max_handshakes_per_second;
    }

    let limiter_period = Duration::from_secs_f64(1.0 / max_handshakes_per_second as f64);
    let rate_limiter = Arc::new(Mutex::new(interval(limiter_period)));

    for _ in 0..cli.concurrently {
        let local_tls_config = tls_config.clone();
        let throttle = Arc::clone(&rate_limiter);
        let now = Instant::now();
        let host = endpoint.ip();
        tasks.spawn(async move {
            let mut results: Vec<tls::TlsDuration> = Vec::new();
            loop {
                let result = tls::handshake_with_timeout(
                    host,
                    endpoint.port(),
                    is_smtp,
                    local_tls_config.clone(),
                    cli.timeout_ms,
                )
                .await;

                if cli.max_handshakes_per_second > 0 {
                    throttle.lock().await.tick().await;
                }

                if result.is_err() {
                    println!("error: {:?}", result.err().unwrap().to_string());
                    continue;
                }

                results.push(result.unwrap());

                println!(
                    "handshake/tcp_connect in ms -> {}/{}",
                    results.last().unwrap().handshake.as_millis(),
                    results.last().unwrap().tcp_connect.as_millis()
                );

                if now.elapsed().as_secs() >= cli.duration {
                    break;
                }
            }

            results
        });
    }

    let data = tasks.join_all().await;
    println!("data: {:?}", data);
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
