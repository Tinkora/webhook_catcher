use std::fs;
use tempfile::tempdir;
use webhook_catcher::capture_request;
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
