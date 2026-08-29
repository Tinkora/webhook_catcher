# webhook-catcher

Local HTTP capture for webhook development. It stores bounded request metadata
and bodies and deduplicates retries by `X-Delivery-ID` or `X-GitHub-Delivery`.

[简体中文](README.zh-CN.md)

```bash
cargo run -- --listen 127.0.0.1:8787 --output ./captures
curl -i -X POST http://127.0.0.1:8787/hook -H 'X-Delivery-ID: demo-1' -d '{"ok":true}'
```

New deliveries return `201 Created`; duplicate IDs return `200 OK` without
overwriting the first payload. This is a localhost development utility, not a
public relay: it does not verify signatures, redact secrets, forward requests,
or provide TLS. Protect and delete captures, which may contain credentials.

## Development

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Support

[Support Tinkora on Ko-fi](https://ko-fi.com/tinkora)

MIT license; see [LICENSE](LICENSE).
