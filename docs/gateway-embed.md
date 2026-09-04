# Gateway embed (VocaWin Settings)

Opt-in Settings group that can start a **local** VocaGateway Compose project on Windows (Docker Desktop + WSL2). This is a pair helper for phones and other Voca clients. VocaWin dictation does **not** route through Gateway.

Contract source: VocaHQ frozen embed notes (`FROZEN.md`). Pin and paths below must stay aligned with that contract.

## Pin (one place)

| Constant | Value |
| --- | --- |
| Release tag | `v0.1.0` |
| Image | `ghcr.io/vocahq/vocagateway:v0.1.0` |
| Rust constants | `GATEWAY_RELEASE_TAG` / `GATEWAY_IMAGE` in `src-tauri/src/gateway.rs` |
| Compose file | `src-tauri/gateway/compose.yaml` (written into app data on Start) |

Do not use floating `latest`.

If GHCR has no public image for that tag yet, fall back to a shallow clone of `VocaHQ/vocagateway` at tag `v0.1.0` into the gateway data dir and `docker compose … up -d --build`. Prefer image pull when the registry publish exists.

## Compose

- Project name: `vocagateway` (`docker compose -p vocagateway`)
- File: `%APPDATA%\com.vocahq.vocawin\gateway\compose.yaml`
- Env: `%APPDATA%\com.vocahq.vocawin\gateway\.env` (token mode restricted on Unix; never commit)
- Service: `gateway` only (CPU). No CUDA/Vulkan profiles in MVP.
- Start: `docker compose -p vocagateway -f <dir>/compose.yaml --env-file <dir>/.env up -d`
- Stop: `… down` **without** `--volumes`

`.env` keys written by VocaWin:

- `VOCAGATEWAY_TOKEN` (≥32 hex chars)
- `VOCAGATEWAY_PUBLISH_HOST=0.0.0.0`
- `VOCAGATEWAY_PUBLISH_PORT=8765`
- `VOCAGATEWAY_PUBLIC_URL` (mandatory non-loopback under Docker Desktop)
- `VOCAGATEWAY_IMAGE` (pinned)

## Health and pairing

| Check | Meaning |
| --- | --- |
| `GET http://127.0.0.1:8765/health/live` | Process up |
| `GET …/health/ready` | Ready for dictation (`503` = not yet) |
| Pairable | live + non-loopback `PUBLIC_URL` + token (pairable **before** Ready) |
| Pairing | `Authorization: Bearer <token>` `GET /v1/admin/pairing?url=…` (Rust only; UI gets URL + QR, not the raw payload/token) |
| QR | Prefer `GET /v1/admin/pairing/qr.svg` (sanitized before Settings HTML) |

Never put `127.0.0.1` / `localhost` in the QR or public URL.

## Honesty

Gateway is **not on-device**. Audio that uses it leaves this PC for the container. Docs: [vocagateway.vocahq.com](https://vocagateway.vocahq.com). Store/MSIX and code signing stay out of scope.
