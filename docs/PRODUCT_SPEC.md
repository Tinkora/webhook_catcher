# Webhook Catcher Product Specification

[简体中文](PRODUCT_SPEC.zh-CN.md)

## Problem

Webhook-driven agents and automations are difficult to debug when the original
HTTP request disappears into application logs. GitHub documents that webhook
URLs cannot point directly to localhost, recommends forwarding during local
testing, and requires a 2xx response within 10 seconds. GitHub CLI forwarding
also supports only repository and organization webhooks and permits only one
active forwarder per repository or organization.

Duplicate delivery is a real agent reliability problem. For example,
[Archon #1951](https://github.com/coleam00/Archon/issues/1951) reports a
duplicate GitHub webhook running the same AI workflow twice because ingestion
lacked idempotency.

References:

- [GitHub webhook troubleshooting](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/troubleshooting-webhooks)
- [GitHub CLI webhook forwarding](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/using-the-github-cli-to-forward-webhooks-for-testing)
- [Testing webhooks locally](https://docs.github.com/en/webhooks/testing-and-troubleshooting-webhooks/testing-webhooks)

## Objective

`webhook-catcher` is a small, local receiver that preserves the exact bounded
body and request metadata for inspection. When a sender provides
`X-Delivery-ID` or `X-GitHub-Delivery`, retries with that ID are acknowledged
without overwriting the first capture.

The alpha succeeds when a developer can connect an existing forwarding tool to
a loopback listener, inspect durable capture files, and exercise delivery-ID
idempotency without deploying a service or database.

## Interface

```text
webhook-catcher [OPTIONS]

Options:
  --listen <IP:PORT>          Loopback address (default: 127.0.0.1:8787)
  --output <DIRECTORY>        Capture directory (default: captures)
  --max-body <BYTES>          Maximum body size (default: 1 MiB)
  --read-timeout-ms <MS>      Per-connection I/O timeout (default: 5,000)
```

Each accepted delivery creates `<delivery-id>.json` and
`<delivery-id>.body`. If no supported delivery header exists, the tool creates
a unique local capture ID and does not deduplicate the request.

## HTTP behavior

- Accept HTTP/1.0 and HTTP/1.1 requests with a valid `Content-Length` or an
  empty body when `Content-Length` is absent.
- Read headers and the declared body across TCP packet boundaries.
- Reject headers larger than 16 KiB, bodies above `--max-body`, conflicting
  length/delivery headers, invalid delivery IDs, and transfer encodings.
- Return `201 Created` for a new capture and `200 OK` for a complete existing
  delivery ID without overwriting it.
- Bind only to IPv4 or IPv6 loopback addresses and time out stalled clients.

## Storage and trust boundary

Headers and bodies are untrusted and may contain credentials. On Unix, newly
created capture directories use mode `0700` and files use `0600`. Existing
directory permissions remain the operator's responsibility. The tool refuses
unsafe delivery IDs, does not follow capture-file symlinks, and never reports a
partial file pair as a successful duplicate.

Captured content is not redacted or encrypted. Users must protect and delete
it. The receiver is a debugging aid, not a security boundary.

## Non-goals

- No public ingress, tunnel, forwarding, TLS, hosted service, or web UI.
- No signature verification, authentication, secret redaction, or encryption.
- No replay, delay/failure injection, retry queue, or provider-specific parser.
- No chunked request bodies or persistent HTTP connections in the alpha.
- No claim of production-grade concurrency or webhook processing.

Tools such as GitHub CLI, smee, ngrok, and Hookdeck solve ingress, forwarding,
or hosted operations. `webhook-catcher` deliberately complements them with a
small local capture and idempotency surface.

## Verification

1. Split TCP reads reconstruct the exact `Content-Length` body.
2. Header/body limits and unsupported framing fail before files are created.
3. A repeated supported delivery ID never overwrites the first complete pair.
4. Requests without an ID receive different local IDs and are never deduplicated.
5. Invalid IDs, partial existing captures, and non-regular files fail safely.
6. English-first and Simplified Chinese docs, locked Rust checks, CodeQL,
   dependency audit, SBOM, checksums, and provenance gate each release.
