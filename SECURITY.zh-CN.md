# 安全策略

[English](SECURITY.md)

Webhook Catcher 会原样保存不可信的请求 header 和正文。捕获内容可能包含 token、cookie、
签名、个人数据或其他 secret。工具不认证发送方、不验证签名、不脱敏、不加密文件，也
不构成网络安全边界。请只在可信开发机器上使用，并在完成调试后删除捕获。

请使用 [GitHub 私密漏洞报告](https://github.com/Tinkora/webhook_catcher/security/advisories/new)。
不要在公开 Issue 中提交 secret、漏洞利用细节或真实 webhook payload。

设计承诺：只绑定 IPv4/IPv6 loopback 地址；限制 header/正文缓冲并让停滞连接超时；
拒绝不安全 delivery ID 和非普通捕获路径；在 Unix 上以私有权限新建目录和文件；相同
delivery ID 的完整捕获永不被覆盖。
