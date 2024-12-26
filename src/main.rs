use clap::Parser;
use std::thread::available_parallelism;
use tokio::{task::JoinSet, time::Instant};

mod tls;

/// Simple program to greet a person
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Endpoint to run TLS benchmark against
    #[arg(short, long, value_parser = parse_endpoint)]
    endpoint: Option<(String, u16)>,

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

    /// Max concurrently running, defaults to available_parallelism
    #[arg(short, long, default_value_t = available_parallelism().unwrap().get())]
    concurrently: usize,
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

fn parse_endpoint(input: &str) -> Result<(String, u16), String> {
    let mut split = input.split(':');
    let host = split.next().ok_or("missing host")?.to_string();
    let port = split
        .next()
        .ok_or("missing port")?
        .parse::<u16>()
        .map_err(|_| "invalid port")?;
    Ok((host, port))
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut tls_config = tls::tls_config(Some(args.zero_rtt), Some(&[&rustls::version::TLS12]));
    match args.tls_version {
        TlsVersion::Tls13 => {
            tls_config = tls::tls_config(Some(args.zero_rtt), Some(&[&rustls::version::TLS13]));
        }
        _ => {}
    }

    let mut is_smtp = false;
    match args.protocol {
        Protocol::Smtp => is_smtp = true,
        _ => {}
    }

    let now = Instant::now();
    let endpoint = args.endpoint.unwrap();

    let mut tasks = JoinSet::new();

    for _ in 0..args.concurrently {
        let host = endpoint.clone().0.leak();
        let port = endpoint.1;
        let local_tls_config = tls_config.clone();
        tasks.spawn(async move {
            loop {
                let result = tls::handshake_with_timeout(
                    host,
                    port,
                    is_smtp,
                    local_tls_config.clone(),
                    4000,
                )
                .await
                .unwrap();

                println!(
                    "handshake/tcp_connect in ms -> {}/{}",
                    result.handshake.as_millis(),
                    result.tcp_connect.as_millis()
                );

                if now.elapsed().as_secs() >= args.duration {
                    break;
                }
            }
        });
    }

    tasks.join_all().await;
}
