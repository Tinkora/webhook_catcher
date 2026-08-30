use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    net::TcpStream,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_HEADER_BYTES: usize = 16 * 1024;
static CAPTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    let split = header_end(raw)?;
    if split > MAX_HEADER_BYTES {
        return invalid_data("headers exceed configured limit");
    }
    let head = std::str::from_utf8(&raw[..split])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "headers are not UTF-8"))?;
    let mut lines = head.lines();
    let first = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut parts = first.split_ascii_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let request_path = parts.next().unwrap_or("").to_string();
    let version = parts.next().unwrap_or("");
    if method.is_empty()
        || request_path.is_empty()
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
        || parts.next().is_some()
    {
        return invalid_data("invalid request line");
    }
    let mut headers = Vec::new();
    let mut delivery = None;
    let mut content_length = None;
    for line in lines {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP header"))?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return invalid_data("invalid HTTP header name");
        }
        let key = key.to_ascii_lowercase();
        let value = value.trim().to_string();
        if value.bytes().any(|byte| byte < b' ' && byte != b'\t') {
            return invalid_data("invalid HTTP header value");
        }
        if key == "transfer-encoding" {
            return invalid_data("chunked transfer encoding is not supported");
        }
        if key == "content-length" {
            if content_length.is_some() {
                return invalid_data("multiple Content-Length headers are not supported");
            }
            content_length = Some(value.parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?);
        }
        if key == "x-delivery-id" || key == "x-github-delivery" {
            if delivery.is_some() {
                return invalid_data("multiple delivery IDs are not supported");
            }
            validate_delivery_id(&value)?;
            delivery = Some(value.clone());
        }
        headers.push((key, value));
    }
    let body = &raw[split + 4..];
    if body.len() > max_body {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "body exceeds configured limit",
        ));
    }
    let expected_body = content_length.unwrap_or(0);
    if body.len() != expected_body {
        return invalid_data("request body does not match Content-Length");
    }
    create_private_directory(output)?;
    let id = delivery.unwrap_or_else(generated_capture_id);
    let path = output.join(format!("{id}.json"));
    let body_path = output.join(format!("{id}.body"));
    let metadata_exists = path.try_exists()?;
    let body_exists = body_path.try_exists()?;
    let duplicate = match (metadata_exists, body_exists) {
        (true, true) => {
            ensure_regular_file(&path)?;
            ensure_regular_file(&body_path)?;
            true
        }
        (false, false) => false,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "incomplete capture already exists",
            ));
        }
    };
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
        let data = serde_json::to_vec_pretty(&cap)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        write_private_new(&body_path, body)?;
        if let Err(error) = write_private_new(&path, &data) {
            let _ = fs::remove_file(&body_path);
            return Err(error);
        }
    }
    Ok(cap)
}

pub fn read_http_request(reader: &mut impl Read, max_body: usize) -> io::Result<Vec<u8>> {
    let mut request = Vec::with_capacity(MAX_HEADER_BYTES.min(4096));
    let header_length = loop {
        if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break end;
        }
        if request.len() >= MAX_HEADER_BYTES + 4 {
            return invalid_data("headers exceed configured limit");
        }
        let mut buffer = [0_u8; 4096];
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return invalid_data("missing HTTP header terminator");
        }
        request.extend_from_slice(&buffer[..count]);
    };

    let head = std::str::from_utf8(&request[..header_length])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "headers are not UTF-8"))?;
    let content_length = parse_content_length(head)?;
    if content_length > max_body {
        return invalid_data("body exceeds configured limit");
    }
    let total_length = header_length + 4 + content_length;
    if request.len() > total_length {
        request.truncate(total_length);
    }
    while request.len() < total_length {
        let remaining = total_length - request.len();
        let mut buffer = [0_u8; 4096];
        let count = reader.read(&mut buffer[..remaining.min(4096)])?;
        if count == 0 {
            return invalid_data("request body ended before Content-Length");
        }
        request.extend_from_slice(&buffer[..count]);
    }
    Ok(request)
}

fn header_end(raw: &[u8]) -> io::Result<usize> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP header terminator"))
}

fn parse_content_length(head: &str) -> io::Result<usize> {
    let mut length = None;
    for line in head.lines().skip(1) {
        let Some((key, value)) = line.split_once(':') else {
            return invalid_data("invalid HTTP header");
        };
        if key.eq_ignore_ascii_case("transfer-encoding") {
            return invalid_data("chunked transfer encoding is not supported");
        }
        if key.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return invalid_data("multiple Content-Length headers are not supported");
            }
            length = Some(value.trim().parse::<usize>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid Content-Length")
            })?);
        }
    }
    Ok(length.unwrap_or(0))
}

fn validate_delivery_id(delivery_id: &str) -> io::Result<()> {
    if delivery_id.is_empty()
        || delivery_id.len() > 128
        || !delivery_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return invalid_data("invalid delivery ID");
    }
    Ok(())
}

fn generated_capture_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = CAPTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("capture-{timestamp}-{sequence}")
}

fn invalid_data<T>(message: &str) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, message))
}

fn create_private_directory(path: &Path) -> io::Result<()> {
    if path.try_exists()? {
        return Ok(());
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn write_private_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn ensure_regular_file(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capture path is not a regular file",
        ));
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
            output
        },
    )
}
pub fn respond(mut stream: TcpStream, status: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
