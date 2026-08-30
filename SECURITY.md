# Security Policy

[简体中文](SECURITY.zh-CN.md)

## Scope

Webhook Catcher stores untrusted request headers and bodies exactly as received.
Captures may contain tokens, cookies, signatures, personal data, or other
secrets. It does not authenticate senders, verify signatures, redact data,
encrypt files, or provide a network security boundary. Use it only on a trusted
development machine and delete captures when finished.

## Reporting

Please use [GitHub private vulnerability reporting](https://github.com/Tinkora/webhook_catcher/security/advisories/new)
when available. Do not open a public issue containing secrets, exploit details,
or real webhook payloads.

## Design commitments

- Bind only to IPv4 or IPv6 loopback addresses.
- Bound header and body buffering and time out stalled connections.
- Reject unsafe delivery IDs and non-regular capture paths.
- Create new Unix capture directories and files with private permissions.
- Never overwrite an existing complete capture for the same delivery ID.
