use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Endpoint to run TLS benchmark against
    #[arg(short, long, value_parser = parse_endpoint)]
    endpoint: Option<(String, u16)>,

    /// Protocol to use when runing TLS benchmark
    #[arg(short, default_value = "tcp")]
    protocol: String,
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

    match args.protocol.as_str() {
        "tcp" => println!("tcp {}:{}", endpoint.0, endpoint.1),
        "smtp" => println!("smtp {}:{}", endpoint.0, endpoint.1),
        _ => print!("err"),
    }
}
