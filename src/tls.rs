mod verify;

use rustls::{crypto::aws_lc_rs as provider, pki_types::ServerName, SupportedProtocolVersion};
use std::{
    io::{Error, ErrorKind},
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    time::{timeout, Duration, Instant},
};
use tokio_rustls::{
    rustls::{ClientConfig, RootCertStore},
    TlsConnector,
};

#[derive(Debug)]
pub struct TlsDuration {
    pub tcp_connect: Duration,
    pub handshake: Duration,
}

pub fn tls_config(
    zero_rtt: Option<bool>,
    supported_tls_version: Option<&[&'static SupportedProtocolVersion]>,
) -> ClientConfig {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut config = ClientConfig::builder_with_protocol_versions(
        supported_tls_version.unwrap_or(rustls::ALL_VERSIONS),
    )
    .with_root_certificates(root_cert_store)
    .with_no_client_auth();

    config
        .dangerous()
        .set_certificate_verifier(Arc::new(verify::NoCertificateVerification::new(
            provider::default_provider(),
        )));

    config.enable_early_data = zero_rtt.unwrap_or(false);
    config
}

async fn handshake(
    host: IpAddr,
    port: u16,
    is_smtp: bool,
    tls_config: ClientConfig,
) -> Result<TlsDuration, Error> {
    let tcp_now = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await?;
    if is_smtp {
        let mut buffer = vec![0; 1024];
        stream.read_buf(&mut buffer).await?;

        stream
            .write_all(&format!("EHLO {}\r\n", host).into_bytes())
            .await?;
        stream.flush().await?;
        stream.read_buf(&mut buffer).await?;

        stream.write_all(b"STARTTLS\r\n").await?;
        stream.flush().await?;

        stream.read_buf(&mut buffer).await?;

        let str_buffer = String::from_utf8_lossy(&buffer);
        if !str_buffer.contains("220") {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "STARTTLS seems to be unsupported",
            ));
        }
    }
    let tcp_connect_duration = tcp_now.elapsed();

    let domain = ServerName::from(host);

    let tls_connector = TlsConnector::from(Arc::new(tls_config));
    let handshake_now = Instant::now();
    let mut tls_stream = tls_connector.connect(domain.to_owned(), stream).await?;
    let handshake_duration = handshake_now.elapsed();

    tls_stream.shutdown().await?;

    Ok(TlsDuration {
        tcp_connect: tcp_connect_duration,
        handshake: handshake_duration,
    })
}

async fn handshake_with_timeout(
    host: IpAddr,
    port: u16,
    is_smtp: bool,
    tls_config: ClientConfig,
    timeout_ms: u64,
) -> Result<TlsDuration, Error> {
    let handshake_timeout = timeout(
        Duration::from_millis(timeout_ms),
        handshake(host, port, is_smtp, tls_config),
    );
    handshake_timeout.await?
}

pub async fn tls_handshaker(
    endpoint: SocketAddr,
    timeout_ms: u64,
    is_smtp: bool,
    tls_config: ClientConfig,
    tx_result: mpsc::UnboundedSender<Result<TlsDuration, Error>>,
) {
    let result = handshake_with_timeout(
        endpoint.ip(),
        endpoint.port(),
        is_smtp,
        tls_config,
        timeout_ms,
    )
    .await;

    let _ = tx_result.send(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_zero_rtt() {
        let config = tls_config(Some(true), Some(&[&rustls::version::TLS12]));
        assert!(config.enable_early_data);
    }

    #[tokio::test]
    async fn test_handshake_connection_refused() {
        let config = tls_config(Some(false), Some(&[&rustls::version::TLS12]));
        let result =
            handshake_with_timeout("127.0.0.1".parse().unwrap(), 8000, false, config, 10).await;
        assert!(result.is_err());
        assert!(&result
            .err()
            .unwrap()
            .to_string()
            .contains("Connection refused"));
    }
}
