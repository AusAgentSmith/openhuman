# Run it today — local-OAuth build

A walkthrough for taking the `feat/local-oauth-no-backend` branch from a fresh
clone to a working desktop app. Single-user, local-first: no OpenHuman backend,
no app login, no Composio aggregator.

## TL;DR

```bash
# 1. Build everything (run from the repo root)
pnpm install
pnpm dev:app
```

`pnpm dev:app` chains `pnpm tauri:ensure` automatically — it installs
the vendored CEF-aware Tauri CLI on first run, then boots Vite + Tauri.

Once the Tauri window opens:

1. Go to **Settings → AI** → add your OpenAI key under the seeded
   `openai` provider entry.
2. *(Optional)* Go to **Settings → Developer Options → Composio Routing
   (Direct Mode)** and paste your Composio API key for long-tail
   integrations such as Discord, Slack, Notion, and non-native Gmail
   actions.
3. *(Optional)* Run the loopback OAuth CLI for Google + GitHub if you
   want native Gmail/Calendar/Drive/GitHub tool execution.

You can do this entirely from the UI. The CLI flows below are the
scripted equivalents — useful for headless testing, but not required.

---

## Deployment topologies

There are two ways to run this fork. Choose one before you start.

### In-process mode (local)

The upstream default. `openhuman-core` runs as a tokio task inside the
Tauri host process — no separate server, no network hop. Suitable for a
single machine that stays on all day, or for offline-first use.

Selected in `BootCheckGate` on first launch by choosing **Local**.

Trade-offs: the core shuts down when you close the app window; only one
client at a time; all state lives in `~/.openhuman` on the local machine.

### Server + client mode (cloud) — recommended for this fork

`openhuman-core` runs headless in a Docker container on any reachable
host (your homelab, a VPS, a second machine on Tailscale). The desktop
app is a thin client that connects to it over HTTP with a bearer token.

Selected in `BootCheckGate` on first launch by choosing **Cloud**, then
entering the server URL and bearer token.

Benefits over in-process mode:
- Core keeps running when the app is closed or the laptop sleeps.
- Multiple desktop clients (different machines) can connect to the same
  core instance.
- Clean separation: state and long-running tasks live on a stable server;
  the desktop is just a view.

The server exposes port **7788**; the single RPC endpoint is
`http://<host>:7788/rpc`. The bearer token is written to
`<workspace>/core.token` inside the container on first start.

The desktop picker accepts either the server origin (`http://<host>:7788`
or `https://openhuman.example.com`) or the full RPC endpoint
(`.../rpc`). It normalizes origin-only input to `/rpc`, stores the URL
and bearer token on the client device, and configures the Tauri host to
use that remote core. In Cloud mode the embedded in-process core is not
started; in Local mode the remote URL/token are cleared and the embedded
core is started on loopback.

---

## Running the server (Docker)

A reference compose file lives at `deploy/node-b/docker-compose.yml` in
this repo. It defines one service (`openhuman-core`), a named volume
(`openhuman-workspace` → `/home/openhuman/.openhuman`), and exposes port
7788.

Bind port 7788 to loopback when a reverse proxy on the same host serves
HTTPS, or to a reachable private interface when clients connect directly
over LAN/Tailscale. Do not expose bearer-authenticated HTTP on the public
internet without TLS in front of it.

**Start the server:**

```bash
# From the directory containing docker-compose.yml
docker compose up -d
```

**Get the bearer token** (required to connect the desktop client):

```bash
docker exec openhuman-core cat /home/openhuman/.openhuman/core.token
```

**Connect the desktop client:**

1. Launch the Tauri app (`pnpm dev:app`).
2. On first run, `BootCheckGate` asks how to connect. Choose **Cloud**.
3. Enter the server URL, e.g. `http://<host>:7788`, and paste the token.
4. The app validates the connection and proceeds to `/home`.

Installed release artifacts do not read the repository `.env` files.
For `.deb`, `.dmg`, `.msi`, or `.AppImage` builds, use the first-launch
runtime picker (or clear the stored mode and pick again) instead of
expecting `OPENHUMAN_CORE_RPC_URL` from your dev shell to be present.

**Version sync:** the desktop app and the server image must be on the
same version — the boot check enforces an exact match. App version is in
`app/package.json`; server version is baked into the image from
`CARGO_PKG_VERSION` at build time.

**Check server health:**

```bash
curl http://<host>:7788/health/live
curl http://<host>:7788/health/ready
```

`/health/live` is the public liveness probe clients should poll to
detect a dead or unreachable core. It returns `ok`, `service`, `probe`,
`status`, `version`, `pid`, `uptime_seconds`, `checked_at`, `checks`,
and endpoint hints. `/health/ready` returns the same shape but reports
whether authenticated JSON-RPC is ready (`checks.rpc_dispatch` and
`checks.rpc_auth`) plus server-runtime readiness signals for capability
inventory, scheduler registration, provider sync registration, and queue
backlog visibility. The payload also includes `runtime.scheduler`,
`runtime.provider_sync`, `runtime.queue_backlog`, and
`runtime.client_sessions` snapshots so operators can distinguish a live
process from one that is actually ready for durable work. Use it before
enabling a desktop, web, or native mobile client session. `/health`
remains a backwards-compatible liveness alias.

**Create a device-scoped client token:**

```bash
curl -s http://<host>:7788/rpc \
  -H "Authorization: Bearer <bootstrap-core-token>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"openhuman.security_client_sessions_create","params":{"label":"phone"}}'
```

The returned `token` is shown once. Store that on the client instead of
the bootstrap `core.token`. Revoke lost devices with
`openhuman.security_client_sessions_revoke`; provider OAuth/API tokens
remain server-side in the workspace `AuthService`.

**View logs:**

```bash
docker logs openhuman-core
```

---

## What the build expects

| Component | How |
| --- | --- |
| **Default LLM** | OpenAI Responses API (`/v1/responses`) with `gpt-5.4` and `reasoning.effort = "medium"`. Set via `DEFAULT_MODEL = "openai:gpt-5.4"`. |
| **Auth storage** | Encrypted-on-disk `AuthService` (`<workspace>/auth-profiles.json`). |
| **OAuth providers** | Google + GitHub via the loopback flow (`127.0.0.1:<random>/oauth/callback`). Never touches a third party. |
| **Composio direct mode** | Backend proxy is gone. Composio v3 calls use the user's own API key from `AuthService`; `composio_authorize` lazily creates missing managed auth configs before opening Composio's hosted OAuth URL. Native dispatch still bypasses Composio for covered Gmail / Calendar / Drive / GitHub slugs. |
| **App login** | Removed. `/` redirects straight to `/home`. |

---

## Step 1 — Install dependencies

```bash
pnpm install
```

The Tauri shell needs the vendored CEF-aware CLI rather than stock
`@tauri-apps/cli` (the latter produces a bundle that panics inside
CEF's library loader). The `dev:app` and `tauri:build:ui` scripts in
`app/package.json` automatically chain `pnpm tauri:ensure`, which
shells out to `scripts/ensure-tauri-cli.sh` and installs
`app/src-tauri/vendor/tauri-cef/crates/tauri-cli` into `~/.cargo/bin`
the first time it runs.

If you want to run the ensure step explicitly (e.g. before a CI build
or after blowing away your cargo bin dir):

```bash
pnpm --filter openhuman-app tauri:ensure
```

It's idempotent — subsequent calls are a fast no-op once the vendored
CLI is installed.

## Step 2 — Run the desktop app

```bash
pnpm dev:app
```

This builds the Rust core, spins up the Tauri shell, and opens the
desktop window. There is no Tauri sidecar `openhuman-core` process. In
Local mode, the JSON-RPC server is a tokio task inside the GUI process
(see `app/src-tauri/src/core_process.rs`). In Cloud mode, the Tauri host
uses the configured remote server and skips the embedded core.

On first launch you should see the runtime picker. Choose **Local** for
the embedded core or **Cloud** for a remote server. Once the boot check
passes, the app proceeds to `/home`. If you see a blank screen and a
spinning loader after the picker, `CoreStateProvider` is still
bootstrapping; give it a few seconds.

## Step 3 — Configure OpenAI from the UI

1. Open **Settings → AI**.
2. The migration has already seeded a cloud provider with slug
   `openai`, endpoint `https://api.openai.com/v1`, `default_model =
   "gpt-5.4"`.
3. Toggle the OpenAI provider on (or click its API-key chip) and paste
   your `sk-...` key.
4. The chat panel should now respond against `gpt-5.4` via the
   Responses API. The `reasoning.effort = "medium"` field is added
   automatically for any `gpt-5*` / `o1*` / `o3*` / `o4*` model
   (`ResponsesReasoning::default_for` in
   `src/openhuman/inference/provider/compatible_types.rs`).

### CLI equivalent

If you'd rather store the key without launching the UI:

```bash
cargo build --bin openhuman-core
./target/debug/openhuman-core rpc auth_store_provider_credentials \
  --params '{"provider":"openai","token":"sk-..."}'
```

The key lands in the same encrypted `auth-profiles.json` the UI uses.

## Step 4 — *(Optional)* Configure Composio direct mode

You only need this if you want the Skills/Integrations cards or the agent
to use Composio-managed long-tail toolkits. Native Google/GitHub actions
covered by `src/openhuman/oauth/native_dispatch.rs` can run without
Composio, but Discord, Slack, Notion, Jira, and other long-tail toolkits
need a Composio API key.

From the Tauri window:

1. Open **Settings → Developer Options**.
2. Open **Composio Routing (Direct Mode)**.
3. Select **Direct (bring your own API key)**.
4. Paste the API key from your Composio account and click **Save**.

The key lands in the active core's encrypted `AuthService` under
`provider:composio-direct`: on the server workspace in Cloud mode, or in
the local workspace in Local mode. It is not written to `config.toml`.

After the key is saved, open the Skills/Integrations grid and click
**Connect** on a toolkit. The core calls Composio v3 directly. If the
toolkit has no v3 auth config in your tenant yet, the core creates a
managed auth config first and then opens Composio's hosted OAuth URL. The
old v2 fallback returns HTTP 410 and is only kept for compatibility with
older error paths; a fresh direct-mode connection should not require any
manual "create auth config" dashboard step.

For real-time triggers, also open **Settings → Developer Options →
Composio Triggers (Direct Mode)** and configure:

- ngrok static domain, e.g. `abc-123.ngrok-free.dev`
- ngrok authtoken
- local receiver enabled

The receiver HMAC-verifies Composio's v3 webhook envelope and publishes
`DomainEvent::ComposioTriggerReceived` into the same triage/reactor
pipeline as the rest of the agent runtime.

## Step 5 — *(Optional)* Connect Google + GitHub natively

You only need this if you want the agent to call Gmail / Calendar /
Drive / GitHub tools. Without it, the LLM still works fine for plain
chat.

Native OAuth requires a build-time client ID per provider (the user is
on a private/personal fork, so unverified Google clients are
acceptable). Build the connect binary with the IDs baked in:

```bash
OPENHUMAN_GOOGLE_OAUTH_CLIENT_ID=<your-google-oauth-client-id> \
OPENHUMAN_GITHUB_OAUTH_CLIENT_ID=<your-github-oauth-client-id> \
  cargo build --bin oauth-connect
```

Then run it once per provider:

```bash
./target/debug/oauth-connect --provider google
./target/debug/oauth-connect --provider github
```

Each invocation:

1. Spins up a one-shot HTTP server on a random `127.0.0.1` port.
2. Opens the provider's consent URL in your system browser.
3. Captures the redirect, exchanges the code via PKCE, and persists the
   resulting access + refresh tokens to `auth-profiles.json` under
   `google` / `github`.
4. Exits.

Tokens auto-refresh on HTTP 401 via `bearer::AuthedClient` — no manual
re-auth until the provider revokes the refresh token (Google unverified
apps: ~7 days; GitHub: indefinite for classic OAuth, otherwise per the
expiring-OAuth-App policy).

Native tool slugs available without ever touching the deleted OpenHuman
backend:

- **Gmail**: `GMAIL_SEND_EMAIL`, `GMAIL_FETCH_EMAILS` /
  `GMAIL_LIST_MESSAGES`, `GMAIL_LIST_LABELS`,
  `GMAIL_FETCH_MESSAGE_BY_MESSAGE_ID`, `GMAIL_DELETE_EMAIL` /
  `GMAIL_DELETE_MESSAGE`, `GMAIL_MOVE_TO_TRASH` /
  `GMAIL_TRASH_EMAIL`, `GMAIL_ADD_LABEL_TO_EMAIL`
- **Calendar**: `GOOGLECALENDAR_EVENTS_LIST` /
  `GOOGLECALENDAR_FIND_EVENT`, `GOOGLECALENDAR_EVENTS_GET`,
  `GOOGLECALENDAR_CREATE_EVENT`
- **Drive**: `GOOGLEDRIVE_LIST_FILES` / `GOOGLEDRIVE_FIND_FILE`,
  `GOOGLEDRIVE_GET_FILE_METADATA`, `GOOGLEDRIVE_CREATE_FILE` /
  `GOOGLEDRIVE_CREATE_FILE_FROM_TEXT`
- **GitHub**: `GITHUB_USERS_GET_AUTHENTICATED`,
  `GITHUB_CREATE_AN_ISSUE`,
  `GITHUB_LIST_REPOSITORIES_FOR_THE_AUTHENTICATED_USER`

Adding more slugs is a single-arm change in
`src/openhuman/oauth/native_dispatch.rs` plus a typed function in
`src/openhuman/providers_native/`.

## Step 6 — Smoke-test

From the Tauri window:

- Open the chat panel and send "hi" — confirm the response comes back.
- *(If you did Step 4)* connect a Composio-backed toolkit such as Discord
  or Gmail from the Skills/Integrations grid. Expected: a hosted Composio
  OAuth URL opens; the core logs `openhuman.composio_authorize -> ok`.
- *(If you did Step 5)* ask the agent to "list my next 3 calendar
  events" or "create a GitHub issue in `<owner>/<repo>` titled foo" —
  confirm it executes via native dispatch (logs prefixed
  `[bearer]` / `[oauth]` in `target/debug-logs/`).
- Try an unwired slug (e.g. `NOTION_SEARCH`) — confirm the agent
  routes through Composio direct mode when a Composio API key is present,
  or surfaces a clear missing-Composio-key error when it is not.

---

## Troubleshooting

### "no cloud provider configured for slug 'openai'"

The migration didn't run, or you launched against a pre-existing
`config.toml` from before the refactor. Easiest fix: delete the file
and let the migration re-seed:

```bash
rm ~/.openhuman/config.toml   # or wherever your workspace lives
```

Then restart the Tauri app.

### "encryption key on this device no longer matches"

A prior login dropped encrypted state that the new build can't read.
Use **Settings → Advanced → Clear app data** (or remove the workspace
dir manually) and restart.

### Tauri panic in `cef::library_loader::LibraryLoader::new`

The stock `@tauri-apps/cli` ran instead of the vendored one. Re-run
`pnpm --filter openhuman-app tauri:ensure` and rebuild.

### gpt-5.4 returns 404 / "model not found"

OpenAI hasn't enabled that model ID on your account yet, or the name
drifted. Swap `DEFAULT_MODEL` in
`src/openhuman/config/schema/types.rs:30` to whichever model your key
has access to (e.g. `"openai:gpt-5"`, `"openai:gpt-4.1"`), rebuild, and
restart. The reasoning-effort field auto-skips for non-reasoning
families.

### "[composio-direct] authorize failed: No auth config found"

That message means you are running an older core. Current direct mode
creates a managed v3 auth config lazily when a toolkit such as Gmail or
Discord has none in your Composio tenant, then requests the hosted connect
URL. Rebuild/restart the desktop or server core and try the connection
again. If the message changes to `auth config create failed`, Composio
rejected the API key or toolkit slug; check **Settings → Developer
Options → Composio Routing (Direct Mode)** and verify the key still works
in your Composio account.

---

## What's NOT working yet

Frontend pages that were tightly coupled to backend-only domains
(rewards, invites, billing, team) still
render in the app but their backing RPCs error out. They're harmless
— just don't expect rewards or billing to do anything. Phase 6 of
`tasks/todo.md` covers replacing or deleting each.
