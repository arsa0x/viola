use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use tokio::{
    net::{TcpStream, lookup_host},
    time::timeout,
};
use viola_core::Context;
use viola_macros::command;
use whatsapp_rust::anyhow;

const ENDPOINT: &str = "g.whatsapp.net:443";
const DEFAULT_PROBES: usize = 3;
const MAX_PROBES: usize = 10;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[command(
    triggers = ["ping", "p"],
    category = "tools",
    description = "Check WhatsApp connectivity latency",
)]
async fn ping(ctx: Context) -> anyhow::Result<()> {
    let probes = parse_probe_count(&ctx.args);

    let dns_started = Instant::now();

    let addresses: Vec<SocketAddr> = lookup_host(ENDPOINT).await?.collect();

    let dns_latency = dns_started.elapsed();

    let address = addresses
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("DNS returned no addresses"))?;

    let tcp = measure_tcp(address, probes).await;

    let send_started = Instant::now();

    let send = ctx.send().text("Process...").quoted().await?;

    let send_latency = send_started.elapsed();

    let mut text = String::new();

    text.push_str(&format!("*Endpoint*\n`{}`\n\n", address));
    text.push_str(&format!("*DNS*\n{:.2} ms\n\n", duration_ms(dns_latency)));

    match tcp {
        Ok(stats) => {
            text.push_str(&format!(
                "*TCP Connectivity*\n\
                 Success: {}/{}\n\
                 Loss: {:.0}%\n\
                 Min: {:.2} ms\n\
                 Avg: {:.2} ms\n\
                 Max: {:.2} ms\n\
                 Jitter: {:.2} ms\n\n",
                stats.successes,
                stats.attempts,
                stats.loss_percent(),
                stats.min_ms(),
                stats.avg_ms(),
                stats.max_ms(),
                stats.jitter_ms(),
            ));
        }

        Err(error) => {
            text.push_str(&format!(
                "*TCP Connectivity*\n\
                 Status: ❌ Failed\n\
                 Error: `{}`\n\n",
                error
            ));
        }
    }

    text.push_str(&format!(
        "*WhatsApp Send*\n{:.2} ms",
        duration_ms(send_latency)
    ));

    ctx.edit_message(&send, ctx.send().text(text).quoted().into_message().await?)
        .await?;

    Ok(())
}

fn parse_probe_count(args: &[String]) -> usize {
    args.get(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_PROBES)
        .clamp(1, MAX_PROBES)
}

async fn measure_tcp(address: SocketAddr, attempts: usize) -> anyhow::Result<TcpStats> {
    let mut samples = Vec::with_capacity(attempts);
    let mut failures = 0usize;

    for _ in 0..attempts {
        let started = Instant::now();

        match timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
            Ok(Ok(stream)) => {
                drop(stream);

                samples.push(duration_ms(started.elapsed()));
            }

            Ok(Err(_)) | Err(_) => {
                failures += 1;
            }
        }
    }

    if samples.is_empty() {
        anyhow::bail!("all TCP probes failed");
    }

    Ok(TcpStats {
        attempts,
        successes: samples.len(),
        failures,
        samples,
    })
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

struct TcpStats {
    attempts: usize,
    successes: usize,
    failures: usize,
    samples: Vec<f64>,
}

impl TcpStats {
    fn min_ms(&self) -> f64 {
        self.samples.iter().copied().fold(f64::INFINITY, f64::min)
    }

    fn avg_ms(&self) -> f64 {
        self.samples.iter().sum::<f64>() / self.samples.len() as f64
    }

    fn max_ms(&self) -> f64 {
        self.samples
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn jitter_ms(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        self.samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .sum::<f64>()
            / (self.samples.len() - 1) as f64
    }

    fn loss_percent(&self) -> f64 {
        self.failures as f64 / self.attempts as f64 * 100.0
    }
}
