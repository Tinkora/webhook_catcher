use std::{
    fs,
    io::{self, Cursor, Read},
};
use tempfile::tempdir;
use webhook_catcher::{capture_request, read_http_request};
#[test]
fn captures_and_deduplicates_delivery() {
    let d = tempdir().unwrap();
    let raw = b"POST /hook HTTP/1.1\r\nX-Delivery-ID: abc-1\r\nContent-Length: 7\r\n\r\npayload";
    let a = capture_request(raw, d.path(), 100).unwrap();
    assert!(!a.duplicate);
    let b = capture_request(raw, d.path(), 100).unwrap();
    assert!(b.duplicate);
    assert_eq!(fs::read(d.path().join("abc-1.body")).unwrap(), b"payload");
}
#[test]
fn rejects_oversize() {
    let d = tempdir().unwrap();
    let raw = b"POST / HTTP/1.1\r\n\r\n12345";
    assert!(capture_request(raw, d.path(), 4).is_err());
}

#[test]
fn reads_a_request_body_across_multiple_reads() {
    let raw = b"POST /hook HTTP/1.1\r\nContent-Length: 7\r\n\r\npayload";
    let mut reader = ChunkedReader::new(raw, 3);

    let request = read_http_request(&mut reader, 100).unwrap();

    assert_eq!(request, raw);
}

struct ChunkedReader<'a> {
    inner: Cursor<&'a [u8]>,
    chunk_size: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(bytes: &'a [u8], chunk_size: usize) -> Self {
        Self {
            inner: Cursor::new(bytes),
            chunk_size,
        }
    }
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = buffer.len().min(self.chunk_size);
        self.inner.read(&mut buffer[..length])
    }
}

#[test]
fn rejects_content_length_larger_than_the_body_limit_before_reading_the_body() {
    let raw = b"POST /hook HTTP/1.1\r\nContent-Length: 101\r\n\r\n";
    let error = read_http_request(&mut Cursor::new(raw), 100).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("body exceeds configured limit"));
}

#[test]
fn rejects_unsupported_chunked_transfer_encoding() {
    let raw = b"POST /hook HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n7\r\npayload\r\n0\r\n\r\n";
    let error = read_http_request(&mut Cursor::new(raw), 100).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("chunked transfer encoding"));
}

#[test]
fn rejects_delivery_ids_that_cannot_be_used_as_file_names() {
    let directory = tempdir().unwrap();
    let raw = b"POST /hook HTTP/1.1\r\nX-Delivery-ID: ../same\r\nContent-Length: 0\r\n\r\n";

    let error = capture_request(raw, directory.path(), 100).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("invalid delivery ID"));
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}

#[test]
fn rejects_mismatched_content_length() {
    let directory = tempdir().unwrap();
    let raw = b"POST /hook HTTP/1.1\r\nContent-Length: 8\r\n\r\npayload";

    let error = capture_request(raw, directory.path(), 100).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(error.to_string().contains("Content-Length"));
}

#[test]
fn requests_without_delivery_ids_are_never_deduplicated() {
    let directory = tempdir().unwrap();
    let raw = b"POST /hook HTTP/1.1\r\nContent-Length: 7\r\n\r\npayload";

    let first = capture_request(raw, directory.path(), 100).unwrap();
    let second = capture_request(raw, directory.path(), 100).unwrap();

    assert!(!first.duplicate);
    assert!(!second.duplicate);
    assert_ne!(first.delivery_id, second.delivery_id);
}

#[test]
fn incomplete_capture_is_not_reported_as_a_duplicate() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("abc-1.json"), b"incomplete").unwrap();
    let raw = b"POST /hook HTTP/1.1\r\nX-Delivery-ID: abc-1\r\nContent-Length: 7\r\n\r\npayload";

    let error = capture_request(raw, directory.path(), 100).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
}

#[cfg(unix)]
#[test]
fn capture_files_are_private_to_the_current_user() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().unwrap();
    let directory = root.path().join("captures");
    let raw = b"POST /hook HTTP/1.1\r\nX-Delivery-ID: abc-1\r\nContent-Length: 7\r\n\r\npayload";

    capture_request(raw, &directory, 100).unwrap();

    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(directory.join("abc-1.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(directory.join("abc-1.body"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
