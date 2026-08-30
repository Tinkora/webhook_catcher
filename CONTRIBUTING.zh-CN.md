# 贡献指南

[English](CONTRIBUTING.md)

请把改动聚焦在本地 webhook 捕获和检查。修改 HTTP framing、存储或去重行为前，先增加
结果导向的回归测试。没有被接受的产品决策时，不要增加公网入口、转发、重放、签名验证、
脱敏或 Web UI。

提交前运行：

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny check advisories bans licenses sources
cargo audit --no-yanked
```

提交信息使用英文 Conventional Commits，例如 `fix: read split webhook request bodies`。
代码注释使用英文；用户可见行为需要更新变更日志。
