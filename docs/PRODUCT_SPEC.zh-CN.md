# Webhook Catcher 产品规格

[English](PRODUCT_SPEC.md)

## 问题

Webhook 驱动的 Agent 和自动化系统出现问题时，原始 HTTP 请求经常消失在应用日志中，
导致本地调试困难。GitHub 文档明确说明 webhook URL 不能直接使用 localhost，建议本地
测试时使用转发，并要求接收端在 10 秒内返回 2xx。GitHub CLI 转发还只支持仓库和组织
webhook，且每个仓库或组织同一时间只能有一个转发会话。

重复投递也是实际的 Agent 可靠性问题。例如
[Archon #1951](https://github.com/coleam00/Archon/issues/1951) 报告：由于入口缺少
幂等处理，同一个 GitHub webhook 让 AI 工作流执行了两次。

参考资料：

- [GitHub webhook 故障排查](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/troubleshooting-webhooks)
- [GitHub CLI webhook 转发](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/using-the-github-cli-to-forward-webhooks-for-testing)
- [本地测试 webhook](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/testing-webhooks)

## 目标

`webhook-catcher` 是一个小型本地接收器，用于保存受大小限制的原始正文和请求元数据。
发送方提供 `X-Delivery-ID` 或 `X-GitHub-Delivery` 时，相同 ID 的重试会被确认，但不会
覆盖首次捕获。

Alpha 版本成功标准是：开发者可以把现有转发工具接到 loopback listener，无需部署服务
或数据库就能检查持久化捕获文件，并测试 delivery ID 幂等行为。

## 命令接口

```text
webhook-catcher [OPTIONS]

选项：
  --listen <IP:PORT>          Loopback 地址（默认 127.0.0.1:8787）
  --output <DIRECTORY>        捕获目录（默认 captures）
  --max-body <BYTES>          正文最大字节数（默认 1 MiB）
  --read-timeout-ms <MS>      单连接 I/O 超时（默认 5,000）
```

每次接受的投递创建 `<delivery-id>.json` 和 `<delivery-id>.body`。没有受支持 delivery
header 时，工具生成唯一的本地捕获 ID，不进行去重。

## HTTP 行为

- 接受带有效 `Content-Length` 的 HTTP/1.0、HTTP/1.1 请求；没有该 header 时只接受空正文。
- 跨 TCP 分片完整读取 header 和声明长度的正文。
- 拒绝超过 16 KiB 的 header、超过 `--max-body` 的正文、冲突的长度或 delivery header、
  非法 delivery ID 和 transfer encoding。
- 新捕获返回 `201 Created`；完整存在的相同 delivery ID 返回 `200 OK`，不覆盖原文件。
- 只绑定 IPv4/IPv6 loopback 地址，并为停滞连接设置超时。

## 存储与信任边界

Header 和正文都不可信，也可能包含凭据。Unix 上新建的捕获目录使用 `0700`，文件使用
`0600`；已有目录的权限由操作者负责。工具拒绝不安全的 delivery ID，不跟随捕获文件
符号链接，也不会把残缺文件对当作成功的重复投递。

捕获内容不会脱敏或加密，用户必须妥善保护并及时删除。该接收器只是调试工具，不是
安全边界。

## 非目标

- 不提供公网入口、隧道、转发、TLS、托管服务或 Web UI。
- 不验证签名、不认证、不脱敏 secret，也不加密。
- 不提供重放、延迟/失败注入、重试队列或 provider 专用解析。
- Alpha 不支持 chunked request body 或 HTTP 长连接。
- 不宣称具备生产级并发或 webhook 处理能力。

GitHub CLI、smee、ngrok 和 Hookdeck 负责入口、转发或托管能力；`webhook-catcher` 只补充
小型本地捕获和幂等测试能力。

## 验证标准

1. TCP 分片读取能按 `Content-Length` 还原原始正文。
2. Header/正文超限和不支持的 framing 在创建文件前失败。
3. 重复的受支持 delivery ID 不覆盖首次完整文件对。
4. 无 ID 请求获得不同的本地 ID，永不自动去重。
5. 非法 ID、残缺捕获和非普通文件安全失败。
6. 每次发布由英文默认/简体中文文档、锁定 Rust 检查、CodeQL、依赖审计、SBOM、校验和
   与 provenance 共同把关。
