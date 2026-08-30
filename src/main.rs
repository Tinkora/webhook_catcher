use clap::Parser;
use std::{
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    time::Duration,
};
use webhook_catcher::{capture_request, read_http_request, respond};
#[derive(Parser)]
#[command(
    name = "webhook-catcher",
    version,
    about = "Capture local webhook deliveries for inspection"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8787", value_parser = parse_loopback_address)]
    listen: SocketAddr,
    #[arg(long, default_value = "captures")]
    output: PathBuf,
    #[arg(long, default_value_t = 1_048_576)]
    max_body: usize,
    #[arg(long, default_value_t = 5_000, value_parser = clap::value_parser!(u64).range(100..=60_000))]
    read_timeout_ms: u64,
}
fn main() -> std::io::Result<()> {
    let c = Cli::parse();
    let l = TcpListener::bind(c.listen)?;
    eprintln!("listening on http://{}", c.listen);
    for s in l.incoming() {
        let mut s = s?;
        s.set_read_timeout(Some(Duration::from_millis(c.read_timeout_ms)))?;
        s.set_write_timeout(Some(Duration::from_millis(c.read_timeout_ms)))?;
        match read_http_request(&mut s, c.max_body)
            .and_then(|request| capture_request(&request, &c.output, c.max_body))
        {
            Ok(x) => {
                eprintln!(
                    "{} {} delivery={} duplicate={}",
                    x.method, x.path, x.delivery_id, x.duplicate
                );
                respond(s, if x.duplicate { "200 OK" } else { "201 Created" })?
            }
            Err(e) => {
                eprintln!("request rejected: {e}");
                respond(s, "400 Bad Request")?
            }
        }
    }
    Ok(())
}

fn parse_loopback_address(value: &str) -> Result<SocketAddr, String> {
    let address = value
        .parse::<SocketAddr>()
        .map_err(|_| "listen address must be an IP address and port".to_string())?;
    if !address.ip().is_loopback() {
        return Err("listen address must use an IPv4 or IPv6 loopback address".to_string());
    }
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::parse_loopback_address;

    #[test]
    fn accepts_ipv4_and_ipv6_loopback_addresses() {
        assert!(parse_loopback_address("127.0.0.1:8787").is_ok());
        assert!(parse_loopback_address("[::1]:8787").is_ok());
    }

    #[test]
    fn rejects_non_loopback_and_hostname_addresses() {
        assert!(parse_loopback_address("0.0.0.0:8787").is_err());
        assert!(parse_loopback_address("192.0.2.1:8787").is_err());
        assert!(parse_loopback_address("localhost:8787").is_err());
    }
}
