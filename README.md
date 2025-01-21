# TLS Bench
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/hmilkovi/tls-bench/blob/main/LICENSE
[actions-badge]: https://github.com/hmilkovi/tls-bench/actions/workflows/ci.yml/badge.svg?branch=main
[actions-url]: https://github.com/hmilkovi/tls-bench/actions/workflows/ci.yml

A TLS benchmarking tool for evaluating servers TLS handshake performance

## Usage manual
```console
Usage: tls-bench [OPTIONS] --endpoint <ENDPOINT>

Options:
  -e, --endpoint <ENDPOINT>
          Endpoint to run TLS benchmark against
  -p <PROTOCOL>
          Protocol to use when running TLS benchmark [default: tcp] [possible values: tcp, smtp]
  -t <TLS_VERSION>
          TLS version number [default: tls12] [possible values: tls12, tls13]
  -z, --zero-rtt
          TLS Zero RTT boolean
  -d, --duration <DURATION>
          Duration of benchamrk test in seconds [default: 0]
  -c, --concurrently <CONCURRENTLY>
          Max concurrently running workers, defaults to available_parallelism
      --timeout-ms <TIMEOUT_MS>
          Timeout of tcp connection & tls handshake in miliseconds [default: 500]
  -m, --max-handshakes-per-second <MAX_HANDSHAKES_PER_SECOND>
          Maximum TLS handshakes per seconds [default: 1000]
  -h, --help
          Print help
  -V, --version
          Print version
```

## Output example
```console
⢁ TLS Handshaks: 652 | errors: 0 | throughput 161 h/s | duration 4.0606017s | success ratio 100%                                                     +---------------+------+-------------+----------------------+-----------------+-----------------+------+
| Latencies     | Min  | AVG         | 50th percentile/mean | 95th percentile | 99th percentile | Max  |
+======================================================================================================+
| TLS Handshake | 35ms | 40.630367ms | 40ms                 | 48ms            | 58.96ms         | 66ms |
|---------------+------+-------------+----------------------+-----------------+-----------------+------|
| TCP Connect   | 13ms | 19.888037ms | 19ms                 | 26ms            | 40.49ms         | 47ms |
+---------------+------+-------------+----------------------+-----------------+-----------------+------+
```
