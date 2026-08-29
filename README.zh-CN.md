# webhook-catcher

用于 Webhook 开发的本地 HTTP 捕获工具。它限制请求正文大小，保存元数据和正文，
并按 `X-Delivery-ID` 或 `X-GitHub-Delivery` 对重试去重。

[English README](README.md)

```bash
cargo run -- --listen 127.0.0.1:8787 --output ./captures
curl -i -X POST http://127.0.0.1:8787/hook -H 'X-Delivery-ID: demo-1' -d '{"ok":true}'
```

新投递返回 `201 Created`，重复 ID 返回 `200 OK` 且不会覆盖首次正文。这是本地
开发工具，不是公网中继；不验证签名、不脱敏、不转发请求，也不提供 TLS。捕获内容
可能包含凭据，请妥善保护并及时删除。

## 开发

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## 支持

[在 Ko-fi 支持 Tinkora](https://ko-fi.com/tinkora)

MIT 许可证，见 [LICENSE](LICENSE)。
