# Server Runtime Migration Plan

## Goal

Make the self-hosted `openhuman-core` server the durable runtime so a user can close the desktop app and continue from a mobile client without losing chat, memory, schedules, triggers, provider sync, or agent work.

The desktop and mobile apps should be interfaces to the same server. Desktop-only OS and webview capabilities can remain as optional collectors while the desktop app is open, but the server must not depend on them for core behavior.

## Current State

Cloud mode now connects the desktop to a remote `openhuman-core` endpoint and stops/skips the embedded in-process core. The active RPC URL and bearer token are synchronized into the Tauri host, and frontend RPC/socket traffic resolves against the configured remote endpoint.

The standalone server already owns the main durable runtime pieces:

- HTTP JSON-RPC at `/rpc`
- Socket.IO realtime stream
- controller dispatch and schema registry
- memory/global store initialization
- agent/event-bus subscribers
- cron scheduler
- update scheduler
- Composio periodic sync and trigger subscribers
- provider credentials/config stored in the server workspace
- inference/provider routing

The remaining risk is not the core RPC split anymore. It is feature ownership: several features still depend on the desktop Tauri shell, CEF/CDP webviews, local OS APIs, or a loopback bridge to desktop state.

## Architecture Rule

Use this rule for every feature:

- If it must continue after the desktop app closes, it belongs on the server or in a third-party webhook/API integration.
- If it needs local device state, screen/audio capture, native webviews, OS notifications, keyboard hooks, or local files, it stays client-side as an optional device capability.
- If server logic currently calls back into Tauri through a loopback bridge, replace that path before treating it as mobile-ready.

## Move Or Confirm On Server

These should be server-owned and are either already there or should be verified as server-safe:

| Area | Target owner | Work |
| --- | --- | --- |
| Chat and agent turns | Server | Keep `openhuman.channel_web_chat` and agent bus fully server-side. Mobile should send the same RPC and receive the same socket events. |
| Memory and vault | Server | Server workspace is source of truth. Clients only read/write through RPC. |
| Cron and scheduled jobs | Server | Server scheduler must run independent of desktop lifecycle. Add health/status visibility for mobile. |
| Composio sync | Server | Periodic sync, trigger archive, and webhook receiver should not require desktop. |
| Provider credentials | Server | OAuth/API keys live in server `AuthService`; clients should not hold provider tokens except login/session material for connecting to the server. |
| LLM routing | Server | BYO provider keys and model routing stay in server config. Clients should not call LLM providers directly. |
| Realtime events | Server | Socket.IO is the shared stream for desktop and mobile. Add mobile-safe reconnect/resume semantics if missing. |
| Notifications | Server plus client push | Server decides notification events; mobile/desktop clients render via platform push/local notification adapters. |
| User/session model | Server | Replace raw pasted bearer token with a real client/session model before public mobile use. |

## Replace With Server-Native Integrations

These desktop mechanisms are useful, but they are not durable server runtime. Replace them with provider APIs, webhooks, Composio direct sync, or bot/OAuth integrations where possible.

| Current mechanism | Why it cannot be the server source of truth | Server-native replacement |
| --- | --- | --- |
| CEF/CDP scanners for Slack, Telegram, Discord, WhatsApp | They require a running desktop CEF runtime and local CDP port. They stop when desktop closes. | Official provider APIs, bot tokens, OAuth APIs, Composio sync, or webhook subscriptions. |
| `webview_apis` loopback bridge | Server-side RPC methods proxy back to a Tauri loopback WebSocket. Standalone server has no Tauri shell. | Implement direct server provider clients, or mark those RPCs desktop-only and hide them in mobile/cloud mode. |
| Webview account sessions/cookies | Stored in desktop CEF profile, not available to server/mobile. | OAuth/token-based provider connections stored in server `AuthService`. |
| Webview-origin notification scraping | Tied to embedded webviews and native notification forwarding. | Provider webhook/API polling on server, then push to active clients. |

## Keep Client-Side As Optional Device Capabilities

These should remain in desktop/mobile clients because they are inherently device-local. They can feed the server while active, but the server must degrade cleanly when they are absent.

| Capability | Owner | Server contract |
| --- | --- | --- |
| Screen capture and screen-share picker | Desktop client | Client sends explicit user-approved artifacts/events to server. |
| Meet camera/audio bridge | Desktop client | Optional live-session capability only. No background dependency. |
| Global hotkeys and dictation toggle | Desktop client | Client captures local hotkey and calls server RPC/socket. |
| Local microphone/audio capture | Client | Client streams/uploads audio to server or local STT path by explicit action. |
| Native notifications | Client | Server emits notification intents; client renders when installed/active. |
| iMessage local database scanner | macOS desktop collector | Optional collector that ingests into server while desktop is running. |
| File-system/local app integrations | Client or explicit server mount | Do not assume mobile can access desktop files. |

## Phased Plan

### Phase 1 - Runtime Contract Audit

Produce a machine-checkable inventory of every RPC/controller and frontend feature:

- `server-safe`: works on standalone `openhuman-core`.
- `client-only`: intentionally depends on local device/Tauri/mobile APIs.
- `desktop-collector`: optional desktop feature that can ingest into server.
- `blocked-by-tauri-bridge`: server RPC still depends on Tauri loopback/webview state.

Acceptance criteria:

- Every `openhuman.*` controller has one of the labels above.
- Mobile UI can hide or degrade unsupported features from the server schema/status.
- `blocked-by-tauri-bridge` list is small, explicit, and tracked.

### Phase 2 - Remove Server Dependence On Tauri Bridges

Treat `webview_apis` and CEF/CDP-backed behavior as blockers for server-first operation.

Work:

- Add capability/status RPCs so clients can tell whether a feature is server-safe, desktop-collector, or unavailable.
- Gate `webview_apis_*` RPCs when `OPENHUMAN_WEBVIEW_APIS_PORT` is absent with a typed "desktop collector unavailable" error.
- Stop exposing bridge-backed features as normal server tools in mobile/cloud mode.
- Prefer direct provider clients under `src/openhuman/providers_native/` or Composio-backed flows.

Acceptance criteria:

- Standalone `openhuman-core serve` does not require the Tauri shell for startup, chat, memory, cron, Composio sync, or mobile-safe provider sync.
- Mobile-safe tool discovery never returns tools that require live desktop CEF/CDP.

### Phase 3 - Server-Native Provider Sync

Replace durable webview scraping needs with server-side integrations.

Priority order:

1. Gmail/Google Calendar/Google Drive/GitHub native provider paths.
2. Composio direct sync and webhooks for long-tail providers.
3. Bot/API-token channel integrations for Discord/Telegram/Slack where user-approved.
4. Desktop collectors only for providers with no practical server-side API.

Acceptance criteria:

- Closing the desktop app does not stop configured provider sync for server-native providers.
- Mobile can show connected provider state from the server.
- Desktop collectors are clearly marked as "only while desktop is open".

### Phase 4 - Mobile Client API Surface

Build mobile against the same server runtime instead of a separate local core assumption.

Work:

- Mobile stores server URL/session credentials.
- Mobile calls JSON-RPC through the same method names as desktop.
- Mobile connects to server Socket.IO for chat, agent progress, notifications, and sync status.
- Mobile uses capability metadata to hide desktop-only collectors and OS-specific tools.
- Add push notification registration RPCs if mobile background alerts are required.

Acceptance criteria:

- Start a chat on desktop, close desktop, continue the thread on mobile.
- Scheduled jobs and background sync continue while all clients are closed.
- Mobile reconnect can recover missed thread/task state from server RPC after socket loss.

### Phase 5 - Client/Session Security

The pasted server bearer token is acceptable for early self-hosted testing, but not for a real mobile client.

Work:

- Add named client sessions/devices with revocation.
- Store only device-scoped tokens on clients.
- Keep provider OAuth/API credentials server-side.
- Add TLS-first deployment guidance and reject public plaintext HTTP for non-private hosts.
- Add audit logs for client connection, token creation, and revocation.

Acceptance criteria:

- A lost phone can be revoked without rotating the whole server token.
- Desktop and mobile sessions are visible from server settings/status.
- Provider tokens never leave the server workspace.

### Phase 6 - Operational Readiness

Make Node B the observable production runtime.

Work:

- Add server health/status endpoints that distinguish liveness, readiness, scheduler status, provider sync status, and queue/backlog state.
- Add Docker compose/runbook checks for volume persistence and token location.
- Add backup/restore docs for the server workspace volume.
- Add upgrade flow for the Docker/server binary independent of desktop app updates.

Acceptance criteria:

- Recreating the container with the named volume preserves config, memory, provider credentials, and server token.
- Operators can tell from `/health/ready` or RPC status whether the server is actually ready for clients.
- Desktop app update and server update are separate, documented operations.

## Immediate Next Tasks

1. Add a controller/feature capability inventory and expose it through RPC.
2. Label `webview_apis` as desktop-bridge-backed and hide it from mobile/cloud tool surfaces.
3. Verify standalone server behavior with no Tauri env vars: chat, memory write/read, cron tick, Composio periodic sync, Socket.IO connect.
4. Convert the highest-value webview-scanner dependency to a server-native provider path.
5. Define the mobile session/token flow before shipping mobile beyond private testing.

## Definition Of Done

The migration is done when this is true:

- `openhuman-core` running on Node B continues all durable work when no desktop process exists.
- Desktop and mobile can connect to the same server and see the same threads, memory, provider state, schedules, and agent progress.
- Any feature that requires a local device is visibly optional and does not break server/mobile continuity.
- No server-safe RPC requires a Tauri loopback bridge, CEF profile, desktop cookie jar, local screen/audio handle, or OS-specific database.
