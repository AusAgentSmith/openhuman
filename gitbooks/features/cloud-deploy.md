# Local Core Deployment

The closedhuman fork no longer ships a hosted product backend or a multi-user
cloud deployment path. The desktop app starts `openhuman-core` in-process by
default, and standalone core mode is only for local debugging, automation, or
trusted single-user environments.

Use the root `docker-compose.yml` when you need a headless local core:

```bash
cp .env.example .env
OPENHUMAN_CORE_TOKEN="$(openssl rand -hex 32)" docker compose up -d
curl -fsS http://127.0.0.1:7788/health
```

Clients must call `/rpc` with the local core bearer token. Provider access is
configured directly in `config.toml` and the encrypted credential store:
native OAuth for Google/GitHub, direct Composio mode with the user's Composio
API key, and BYO LLM providers under Settings -> AI.

For Composio, configure the key from a connected desktop client under
**Settings -> Developer Options -> Composio Routing (Direct Mode)**. The key is
stored on the server workspace, so every client connected to that core sees the
same integration tenant. Toolkit connect flows create missing Composio v3
managed auth configs lazily before opening the hosted OAuth URL.

Real-time Composio triggers also need **Settings -> Developer Options ->
Composio Triggers (Direct Mode)**. Point a static ngrok domain at the server's
local webhook receiver and store the ngrok authtoken in the same encrypted
credential store.

Do not expose the standalone core to the public internet. It is a local
single-user service, not an OpenHuman product backend replacement.
