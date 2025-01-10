use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    io,
    net::SocketAddr,
    sync::Arc,
    thread::{available_parallelism, sleep},
    time::Duration,
    u128,
};
use tokio::{
    net,
    sync::{mpsc, Mutex},
    task,
    time::{interval, Instant},
};

mod math;
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
    let (tx, mut rx) = mpsc::unbounded_channel::<Result<tls::TlsDuration, std::io::Error>>();

    let sync_worker = task::spawn_blocking(move || {
        let spinner_style = ProgressStyle::with_template(
            "[{elapsed_precise}] {prefix:.bold.dim} {spinner} {wide_msg}",
        )
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⡈⡐⡠⣀⣁⣂⣄⣌⣔⣤⣥⣦⣮⣶⣷⣿⡿⠿⢟⠟⡛⠛⠫⢋⠋⠍⡉⠉⠑⠡⢁");
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(spinner_style);

        let mut err_count = 0;
        let mut handshakes_count = 0;
        let mut handshake_latencies: Vec<u128> = Vec::new();
        let mut tcp_connect_latencies: Vec<u128> = Vec::new();

        while let Some(data) = rx.blocking_recv() {
            spinner.tick();
            spinner.set_message(format!(
                "TLS Handshaks: {} | errors: {}",
                handshakes_count, err_count
            ));
            if data.is_err() {
                err_count += 1;
                continue;
            }

            handshakes_count += 1;
            let latencies = data.unwrap();
            handshake_latencies.push(latencies.handshake.as_millis());
            tcp_connect_latencies.push(latencies.tcp_connect.as_millis());
        }
        spinner.set_message("calculating stats...");
        spinner.enable_steady_tick(Duration::from_millis(200));
        let handshake_latencies_median = math::median_nonempty(&mut handshake_latencies);
        let tcp_connect_median = math::median_nonempty(&mut tcp_connect_latencies);
        sleep(Duration::from_secs(1));
        spinner.finish_and_clear();
        println!(
            "handshake_latencies_median: {:?}ms",
            handshake_latencies_median
        );
        println!("tcp_connect_median: {:?}ms", tcp_connect_median);
    });

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

    let mut max_handshakes_per_second = 1;
    if cli.max_handshakes_per_second > 0 {
        max_handshakes_per_second = cli.max_handshakes_per_second;
    }

    let limiter_period = Duration::from_secs_f64(1.0 / max_handshakes_per_second as f64);
    let rate_limiter = Arc::new(Mutex::new(interval(limiter_period)));

    let mut tasks = task::JoinSet::new();
    for _ in 0..cli.concurrently {
        let local_tls_config = tls_config.clone();
        let throttle = Arc::clone(&rate_limiter);
        let now = Instant::now();
        let host = endpoint.ip();
        let tx_result = tx.clone();
        tasks.spawn(async move {
            loop {
                let result = tls::handshake_with_timeout(
                    host,
                    endpoint.port(),
                    is_smtp,
                    local_tls_config.clone(),
                    cli.timeout_ms,
                )
                .await;

                let _ = tx_result.send(result);

                if cli.max_handshakes_per_second > 0 {
                    throttle.lock().await.tick().await;
                }

                if now.elapsed().as_secs() >= cli.duration {
                    break;
                }
            }
        });
    }

    tasks.join_all().await;
    drop(tx);
    sync_worker.await?;

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
