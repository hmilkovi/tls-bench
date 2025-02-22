use comfy_table::Table;
use indicatif::{ProgressBar, ProgressStyle};
use std::io;
use tokio::{sync::mpsc, time::Instant};
use tokio_util::sync::CancellationToken;

use crate::math;
use crate::tls;

fn render_stats_table(handshake_latencies: &mut [u128], tcp_connect_latencies: &mut [u128]) {
    assert!(
        !handshake_latencies.is_empty(),
        "List of handshake latencies can not be empty"
    );
    assert!(
        !tcp_connect_latencies.is_empty(),
        "List of tcp connect latencies can not be empty"
    );
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

pub fn show_progress_and_stats(
    duration: u64,
    ramp_up_sec: u64,
    concurrently: usize,
    mut rx: mpsc::UnboundedReceiver<Result<tls::TlsDuration, io::Error>>,
    token: CancellationToken,
) {
    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⡈⡐⡠⣀⣁⣂⣄⣌⣔⣤⣥⣦⣮⣶⣷⣿⡿⠿⢟⠟⡛⠛⠫⢋⠋⠍⡉⠉⠑⠡⢁");
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(spinner_style);

    let mut err_count: u128 = 0;
    let mut handshakes_count: u128 = 0;
    let mut handshake_latencies: Vec<u128> = Vec::new();
    let mut tcp_connect_latencies: Vec<u128> = Vec::new();
    let mut ramp_up_reset_done = false;

    let mut throughput = 0;
    let mut elapsed_secs = 0.0;
    let now = Instant::now();
    while let Some(data) = rx.blocking_recv() {
        spinner.tick();
        elapsed_secs = now.elapsed().as_secs_f32() - ramp_up_sec as f32;
        if (duration > 0 && elapsed_secs >= duration as f32)
            || (duration == 0
                && ramp_up_sec == 0
                && err_count + handshakes_count >= concurrently.try_into().unwrap())
            || (duration == 0 && elapsed_secs >= 0.0 && ramp_up_sec > 0)
        {
            token.cancel();
            break;
        }

        spinner.set_message(format!(
            "TLS handshakes: {} | errors: {} | throughput {} h/s | duration {:.2}s",
            handshakes_count, err_count, throughput, elapsed_secs
        ));

        if data.is_err() {
            err_count += 1;
            continue;
        }

        handshakes_count += 1;

        let mut throughput_elapsed_sec = elapsed_secs;
        if elapsed_secs <= 0.0 {
            throughput_elapsed_sec = elapsed_secs + ramp_up_sec as f32;
        }

        throughput = (handshakes_count as f32 / throughput_elapsed_sec).ceil() as u128;

        if duration > 0 && elapsed_secs >= 0.0 && !ramp_up_reset_done {
            handshakes_count = 0;
            ramp_up_reset_done = true;
        }

        let latencies = data.unwrap();
        handshake_latencies.push(latencies.handshake.as_millis());
        tcp_connect_latencies.push(latencies.tcp_connect.as_millis());
    }

    if ramp_up_sec > 0 {
        elapsed_secs = elapsed_secs + ramp_up_sec as f32;
    }

    spinner.finish_with_message(format!(
        "TLS handshakes: {} | errors: {} | throughput {} h/s | duration {:.2}s | success ratio {}%",
        handshakes_count,
        err_count,
        throughput,
        elapsed_secs,
        handshakes_count as f32 / (err_count + handshakes_count) as f32 * 100.0
    ));
    render_stats_table(&mut handshake_latencies, &mut tcp_connect_latencies);
}
