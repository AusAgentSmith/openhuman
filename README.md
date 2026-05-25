# OpenHuman (fork)

A fork of [tinyhumansai/openhuman](https://github.com/tinyhumansai/openhuman) with a substantially different deployment model and AI backend strategy. Upstream changes are consumed where they make sense; the product direction and infrastructure assumptions diverge significantly.

---

## What's different from upstream

| | Upstream | This fork |
|---|---|---|
| **OpenHuman API** | Required (auth, LLM proxy, billing) | **Removed** — no dependency on `api.openhuman.ai` |
| **Deployment** | Desktop app only (in-process core) | **Local embedded core or self-hosted server + client apps** |
| **AI providers** | Routed through OpenHuman backend | **Direct BYO API keys** — first-class, no proxy |
| **Local LLM** | Ollama (experimental) | Ollama kept as-is, direction TBD |
| **Integrations** | OpenHuman-managed Composio tenant | **Composio direct mode** with the user's own API key |
| **Mobile client** | Not present | Under development — target once server topology is validated |
| **OAuth** | Handled by OpenHuman backend | Native PKCE flows direct to providers |

---

## Architecture

```
┌─────────────────────────────────────────┐
│  Self-Hosted Server  (Docker / VPS)     │
│                                         │
│  openhuman-core  (Rust · JSON-RPC)      │
│  ├── Memory tree + Obsidian vault       │
│  ├── Agent harness + tool surface       │
│  ├── Native OAuth (Google · GitHub)     │
│  ├── Composio direct mode (BYO key)     │
│  └── Direct LLM calls (BYO keys)        │
└──────────────┬──────────────────────────┘
               │  HTTP JSON-RPC (bearer auth)
     ┌─────────┴──────────┐
     │                    │
 Desktop App          Mobile App
 (Tauri + React)      (under development)
 Win · macOS · Linux  iOS · Android
```

The recommended fork topology runs the core as a headless server in a container. Clients are thin: they connect to any reachable `openhuman-core` instance with a bearer token and keep durable state on that server. The desktop app can still run a local embedded core for offline/single-machine use. There is no shared cloud session, no account login, and no telemetry sent off-device.

---

## AI providers

External AI is the primary path. Configure providers under **Settings → AI** with your own API keys:

| Provider | Auth style | Notes |
|---|---|---|
| OpenAI | Bearer | Default primary |
| Anthropic | Anthropic header | Full support |
| OpenRouter | Bearer | Multi-model routing |
| Any OpenAI-compatible endpoint | Bearer / None | LM Studio, vLLM, llama.cpp server, etc. |

Each provider gets a slug, endpoint, and auth style. Workload routing (`chat_provider`, `reasoning_provider`, `agentic_provider`, etc.) maps roles to `<slug>:<model>` strings in `~/.openhuman/config.toml`.

**Ollama** is left as-is from upstream. It works for local inference but its long-term role in this fork is undecided.

---

## Integrations and Composio

Long-tail integrations use **Composio direct mode**. Configure your key in
the desktop app under **Settings → Developer Options → Composio Routing
(Direct Mode)**. The key is stored in the active core's encrypted
credential store: on the server workspace in Cloud mode, or the local
workspace in Local mode.

Connection cards in the Skills/Integrations UI call
`openhuman.composio_authorize`. In direct mode the core looks up a v3 auth
config for the toolkit and, if none exists yet, creates a managed auth
config automatically before asking Composio for the hosted connect URL.
Users should not have to pre-create auth configs in the Composio dashboard
for common managed-auth toolkits such as Gmail or Discord.

Real-time triggers need one more setup step: **Settings → Developer Options
→ Composio Triggers (Direct Mode)**. Paste an ngrok authtoken and static
`*.ngrok-free.dev` domain so Composio can deliver webhooks to the embedded
receiver.

---

## Quick-start (server)

```bash
# Generate a bearer token
openssl rand -hex 32

# Run the core (Docker)
docker run -d \
  --name openhuman-core \
  -p 127.0.0.1:7788:7788 \
  -e OPENHUMAN_CORE_HOST=0.0.0.0 \
  -e OPENHUMAN_CORE_TOKEN=<your-token> \
  -v openhuman-workspace:/home/openhuman/.openhuman \
  ghcr.io/AusAgentSmith/openhuman-core:latest
```

See [`docker-compose.yml`](./docker-compose.yml) for a more complete reference with volume mounts and restart policy. The example binds to loopback, which is appropriate behind a reverse proxy on the same host. For direct LAN/Tailscale access, publish the port on the reachable interface instead.

Point the desktop app at the server URL and paste the token on first launch. Either the server origin (`https://openhuman.example.com` / `http://<server>:7788`) or the explicit RPC endpoint (`.../rpc`) is accepted; the app normalizes origin-only URLs to `/rpc`.

In **Cloud** runtime mode the desktop stores the server URL + token locally, configures the Tauri host to use that remote endpoint, and does not spawn the embedded local core. In **Local** runtime mode the Tauri host starts the embedded in-process core at `http://127.0.0.1:<port>/rpc` with a per-launch token.

---

## Building from source

**Prerequisites**: Git, Node.js 24+, pnpm 10.10.0+, Rust 1.82+ (`rustfmt` + `clippy` components), CMake, Ninja, ripgrep, and platform desktop prerequisites (see upstream docs for platform-specific libs).

```bash
git clone https://github.com/AusAgentSmith/openhuman.git
cd openhuman
git submodule update --init --recursive
pnpm install

# UI only (Vite dev server)
pnpm dev

# Full desktop app (requires vendored CEF tauri-cli)
pnpm dev:app

# Core server binary
cargo build --bin openhuman-core

# Tests
pnpm test          # Vitest (frontend)
pnpm test:rust     # cargo tests + mock API
```

Run `pnpm typecheck` and `cargo clippy -- -D warnings` before committing.

---

## Notable changes vs upstream

- **No OpenHuman API calls** — auth, LLM inference, OAuth handoff, billing, and referral surfaces are all removed or replaced.
- **Native OAuth** — Google (Gmail / Calendar / Drive) and GitHub use PKCE + loopback redirect (RFC 7636 / RFC 8252). No backend proxy involved.
- **Direct provider clients** — Gmail, Google Calendar, Google Drive, and GitHub have native API clients in `src/openhuman/providers/` with auto-refresh-on-401.
- **Composio direct mode** — Composio v3 calls go against the user's own tenant via their personal API key. Connect creates missing v3 managed auth configs lazily, then opens Composio's hosted OAuth URL. The backend-proxy mode is removed.
- **Auth-style-aware endpoint ops** — `test_endpoint` and `list_provider_models` respect the configured auth style (Bearer / Anthropic / None) rather than assuming Bearer for all calls.
- **Delegate agent provider resolution** — sub-agents with a `<slug>:<model>` model pin resolve directly through the provider factory instead of overriding `default_model`.
- **Workload routing** — `agentic_provider`, `reasoning_provider`, `coding_provider`, etc. are independently configurable per-workload in config.
- **TTS via Kokoro** — preferred voice path on Apple Silicon via mlx-audio / kokoro-fastapi. ElevenLabs proxy (required the backend) is effectively dead.
- Various UI cleanups: provider editor auto-save, model picker from live endpoint, Edit button on provider chips.

---

## Upstream sync

Upstream commits from `tinyhumansai/openhuman` are cherry-picked or merged selectively. Changes that assume the OpenHuman backend, the Composio OAuth aggregator, or the upstream billing/referral surface are skipped or adapted. The fork diverges on auth, AI routing, and deployment topology — patches in those areas will not track upstream.

---

## License

[GNU GPL v3](./LICENSE) — same as upstream.
