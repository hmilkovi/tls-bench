use clap::Parser;

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

fn main() {
    let args = Args::parse();

    let endpoint = args.endpoint.unwrap();

    let mut tls_version = "Tls v1.2";
    match args.tls_version {
        TlsVersion::Tls13 => {
            tls_version = "Tls v1.3";
        }
        _ => {}
    }

    match args.protocol {
        Protocol::Tcp => println!("tcp {} {}:{}", tls_version, endpoint.0, endpoint.1),
        Protocol::Smtp => println!("smtp {} {}:{}", tls_version, endpoint.0, endpoint.1),
    }
}
