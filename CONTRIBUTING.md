# Contributing

[简体中文](CONTRIBUTING.zh-CN.md)

Keep changes focused on local webhook capture and inspection. Add an
outcome-focused regression test before changing HTTP framing, storage, or
deduplication behavior. Do not add public ingress, forwarding, replay,
signature verification, redaction, or a web UI without an accepted product
decision.

Run:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

Use English Conventional Commits, for example
`fix: read split webhook request bodies`. Keep code comments in English and
update the changelog for user-visible behavior.
