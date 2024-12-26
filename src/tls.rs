mod verify;

use rustls::crypto::aws_lc_rs as provider;
use rustls::pki_types::ServerName;
use rustls::SupportedProtocolVersion;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

use std::io::{Error, ErrorKind};
use std::str;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration, Instant};

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
        &supported_tls_version.unwrap_or(rustls::ALL_VERSIONS),
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
    host: &str,
    port: u16,
    is_smtp: bool,
    tls_config: ClientConfig,
) -> Result<TlsDuration, Error> {
    let tcp_now = Instant::now();
    let mut stream = TcpStream::connect((host, port)).await?;
    if is_smtp {
        let mut buffer = vec![0; 1024];
        stream.read(&mut buffer).await?;

        stream
            .write_all(&format!("EHLO {}\r\n", host).into_bytes())
            .await?;
        stream.flush().await?;

        stream.read(&mut buffer).await?;

        stream.write_all(b"STARTTLS\r\n").await?;
        stream.flush().await?;

        stream.read(&mut buffer).await?;

        let str_buffer = String::from_utf8_lossy(&buffer);
        if !str_buffer.contains("220") {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "STARTTLS seems to be unsupported",
            ));
        }
    }
    let tcp_connect_duration = tcp_now.elapsed();

    let domain = ServerName::try_from(host);
    if domain.is_err() {
        return Err(Error::new(ErrorKind::NotFound, "host can not be resolved"));
    }

    let tls_connector = TlsConnector::from(Arc::new(tls_config));
    let handshake_now = Instant::now();
    let mut tls_stream = tls_connector
        .connect(domain.unwrap().to_owned(), stream)
        .await?;
    let handshake_duration = handshake_now.elapsed();

    tls_stream.shutdown().await?;

    Ok(TlsDuration {
        tcp_connect: tcp_connect_duration,
        handshake: handshake_duration,
    })
}

pub async fn handshake_with_timeout(
    host: &str,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_zero_rtt() {
        let config = tls_config(Some(true), Some(&[&rustls::version::TLS12]));
        assert_eq!(config.enable_early_data, true);
    }

    #[tokio::test]
    async fn test_handshake_connection_refused() {
        let config = tls_config(Some(false), Some(&[&rustls::version::TLS12]));
        let result = handshake_with_timeout("127.0.0.1", 8000, false, config, 10).await;
        assert!(result.is_err());
        assert!(&result
            .err()
            .unwrap()
            .to_string()
            .contains("Connection refused"));
    }
}
