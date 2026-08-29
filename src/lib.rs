use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    net::TcpStream,
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct Capture {
    pub delivery_id: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body_sha256: String,
    pub body_bytes: usize,
    pub duplicate: bool,
}

pub fn capture_request(raw: &[u8], output: &Path, max_body: usize) -> io::Result<Capture> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing HTTP header terminator")
        })?;
    let head = std::str::from_utf8(&raw[..split])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "headers are not UTF-8"))?;
    let mut lines = head.lines();
    let first = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let request_path = parts.next().unwrap_or("").to_string();
    if method.is_empty() || request_path.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid request line",
        ));
    }
    let mut headers = Vec::new();
    let mut delivery = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().to_string();
            if key == "x-delivery-id" || key == "x-github-delivery" {
                delivery = Some(val.clone());
            }
            headers.push((key, val));
        }
    }
    let body = &raw[split + 4..];
    if body.len() > max_body {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "body exceeds configured limit",
        ));
    }
    fs::create_dir_all(output)?;
    let id = delivery.unwrap_or_else(|| format!("sha256-{}", hex(&Sha256::digest(body))));
    let path = output.join(format!("{}.json", safe_name(&id)));
    let duplicate = path.exists();
    let mut hasher = Sha256::new();
    hasher.update(body);
    let digest = hex(&hasher.finalize());
    let cap = Capture {
        delivery_id: id,
        method,
        path: request_path,
        headers,
        body_sha256: digest,
        body_bytes: body.len(),
        duplicate,
    };
    if !duplicate {
        let data = serde_json::to_vec_pretty(&cap).unwrap();
        fs::write(&path, data)?;
        fs::write(
            output.join(format!("{}.body", safe_name(&cap.delivery_id))),
            body,
        )?;
    }
    Ok(cap)
}
fn safe_name(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn respond(mut stream: TcpStream, status: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
