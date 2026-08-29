use clap::Parser;
use std::{io::Read, net::TcpListener, path::PathBuf};
use webhook_catcher::{capture_request, respond};
#[derive(Parser)]
#[command(
    name = "webhook-catcher",
    version,
    about = "Capture local webhook deliveries for inspection"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8787")]
    listen: String,
    #[arg(long, default_value = "captures")]
    output: PathBuf,
    #[arg(long, default_value_t = 1_048_576)]
    max_body: usize,
}
fn main() -> std::io::Result<()> {
    let c = Cli::parse();
    let l = TcpListener::bind(&c.listen)?;
    eprintln!("listening on http://{}", c.listen);
    for s in l.incoming() {
        let mut s = s?;
        let mut b = vec![0; c.max_body + 16384];
        let n = s.read(&mut b)?;
        b.truncate(n);
        match capture_request(&b, &c.output, c.max_body) {
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
