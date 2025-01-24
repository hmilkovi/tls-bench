use clap::Parser;
use comfy_table::Table;
use indicatif::{ProgressBar, ProgressStyle};
use std::{io, net::SocketAddr, sync::Arc, thread::available_parallelism, time::Duration};
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

fn render_stats_table(handshake_latencies: &mut [u128], tcp_connect_latencies: &mut [u128]) {
    handshake_latencies.sort();
    tcp_connect_latencies.sort();
    let mut table = Table::new();
    let header = vec![
        "Latencies",
        "Min",
        "AVG",
        "50%’ile",
        "95%’ile",
        "99%’ile",
        "99.9%’ile",
        "Max",
    ];
    table
        .set_header(header)
        .add_row(vec![
            String::from("TLS Handshake"),
            format!("{}ms", handshake_latencies[0]),
            format!("{}ms", math::avg(handshake_latencies)),
            format!("{}ms", math::percentile(handshake_latencies, 50.0) as f32),
            format!("{}ms", math::percentile(handshake_latencies, 95.0) as f32),
            format!("{}ms", math::percentile(handshake_latencies, 99.0) as f32),
            format!("{}ms", math::percentile(handshake_latencies, 99.9) as f32),
            format!("{}ms", handshake_latencies.last().unwrap()),
        ])
        .add_row(vec![
            String::from("TCP Connect"),
            format!("{}ms", tcp_connect_latencies[0]),
            format!("{}ms", math::avg(tcp_connect_latencies)),
            format!("{}ms", math::percentile(tcp_connect_latencies, 50.0) as f32),
            format!("{}ms", math::percentile(tcp_connect_latencies, 95.0) as f32),
            format!("{}ms", math::percentile(tcp_connect_latencies, 99.0) as f32),
            format!("{}ms", math::percentile(tcp_connect_latencies, 99.9) as f32),
            format!("{}ms", tcp_connect_latencies.last().unwrap()),
        ]);

    println!("{table}");
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let (tx, mut rx) = mpsc::unbounded_channel::<Result<tls::TlsDuration, std::io::Error>>();

    let sync_worker = task::spawn_blocking(move || {
        let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⡈⡐⡠⣀⣁⣂⣄⣌⣔⣤⣥⣦⣮⣶⣷⣿⡿⠿⢟⠟⡛⠛⠫⢋⠋⠍⡉⠉⠑⠡⢁");
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(spinner_style);

        let mut err_count: u128 = 0;
        let mut handshakes_count: u128 = 0;
        let mut handshake_latencies: Vec<u128> = Vec::new();
        let mut tcp_connect_latencies: Vec<u128> = Vec::new();

        let now = Instant::now();
        let mut throughput = 0;
        while let Some(data) = rx.blocking_recv() {
            spinner.tick();
            let elapsed_secs = now.elapsed().as_secs_f32();
            throughput = (handshakes_count as f32 / elapsed_secs).ceil() as u128;
            spinner.set_message(format!(
                "TLS handshakes: {} | errors: {} | throughput {} h/s | duration {:.2}s",
                handshakes_count, err_count, throughput, elapsed_secs
            ));

            if data.is_err() {
                err_count += 1;
            } else {
                handshakes_count += 1;
                let latencies = data.unwrap();
                handshake_latencies.push(latencies.handshake.as_millis());
                tcp_connect_latencies.push(latencies.tcp_connect.as_millis());
            }
        }
        let duration = now.elapsed().as_secs_f32();
        spinner.set_message(format!(
            "TLS handshakes: {} | errors: {} | throughput {} h/s | duration {}s | success ratio {}%",
            handshakes_count,
            err_count,
            throughput,
            duration,
            handshakes_count as f32/(err_count+handshakes_count) as f32 * 100.0
        ));
        spinner.finish();
        render_stats_table(&mut handshake_latencies, &mut tcp_connect_latencies);
    });

    let mut tls_config = tls::tls_config(Some(cli.zero_rtt), Some(&[&rustls::version::TLS12]));
    if let TlsVersion::Tls13 = cli.tls_version {
        tls_config = tls::tls_config(Some(cli.zero_rtt), Some(&[&rustls::version::TLS13]));
    }

    let mut is_smtp = false;
    if let Protocol::Smtp = cli.protocol {
        is_smtp = true;
    }

    let endpoint: SocketAddr = net::lookup_host(cli.endpoint).await?.next().unwrap();

    let limiter_period = Duration::from_secs_f64(1.0 / cli.max_handshakes_per_second as f64);
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
                if cli.max_handshakes_per_second > 0 {
                    throttle.lock().await.tick().await;
                }

                let result = tls::handshake_with_timeout(
                    host,
                    endpoint.port(),
                    is_smtp,
                    local_tls_config.clone(),
                    cli.timeout_ms,
                )
                .await;

                let _ = tx_result.send(result);

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
