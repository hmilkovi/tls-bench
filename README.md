# TLS Bench
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/hmilkovi/tls-bench/blob/main/LICENSE
[actions-badge]: https://github.com/hmilkovi/tls-bench/actions/workflows/ci.yml/badge.svg?branch=main
[actions-url]: https://github.com/hmilkovi/tls-bench/actions/workflows/ci.yml

A TLS benchmarking tool for evaluating servers TLS handshake performance

## Motivation
This tool was born from the need to load test latencies of TLS-terminating services such as reverse proxies and load balancers.

This tool enables users to:
* Compare latencies for different TLS-terminating balancers or reverse proxies.
* Observe the impact of private key sizes on TLS handshake latencies and handshakes per second.
* Replicate traffic patterns where TCP connections are short-lived, yet TLS is still necessary. This includes scenarios like the "thundering herd" effect, often observed in services handling affiliate links or URL shorteners, where a sudden surge of requests demands rapid TLS handshakes.

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

Usage Example:
```console
tls-bench -e 127.0.0.1:443 -t tls13 -c 10 -d 10 -m 20
```

## Output example
```console
⢁ TLS handshakes: 644 | errors: 0 | throughput 158 h/s | duration 4.073427s | success ratio 100%
+---------------+------+-------------+---------+---------+---------+-----------+------+
| Latencies     | Min  | AVG         | 50%’ile | 95%’ile | 99%’ile | 99.9%’ile | Max  |
+=====================================================================================+
| TLS Handshake | 34ms | 40.989132ms | 40ms    | 47ms    | 57.57ms | 61.357ms  | 62ms |
|---------------+------+-------------+---------+---------+---------+-----------+------|
| TCP Connect   | 13ms | 20.18789ms  | 20ms    | 26ms    | 36ms    | 49ms      | 49ms |
+---------------+------+-------------+---------+---------+---------+-----------+------+
```

## Install

### Pre-compiled executables

Get them [here](https://github.com/hmilkovi/tls-bench/releases).

## Feature Roadmap
Rough sketch of feature roadmap that will be implemented:
- [ ] Create Docker image
- [ ] Create Homebrew formula
- [ ] Prometheus support
- [ ] Add ramp-up
- [ ] Add load patterns
- [ ] Allow usage as library

## License
See [MIT](LICENSE).
