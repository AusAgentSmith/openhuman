---
description: >-
  High-level shape of the OpenHuman system (desktop shell, Rust core, Memory
  Tree, agent loop). Pointer to the deep developer architecture in the repo.
icon: code-branch
---

# Architecture

OpenHuman is open-sourced under GNU GPL3. This page is the high-level shape of the system; the deep developer architecture lives in [deep architecture reference](../architecture.md) in the repo.

## The shape

OpenHuman is a **React + Tauri v2 desktop app** with a **Rust core** that does the heavy lifting.

```
┌──────────────────────────────────────────────────┐
│ Tauri shell (app/src-tauri/) │
│ • windowing, OS integration, core runtime lifecycle │
│ • CEF child webviews for integration providers │
└──────────────────────────────────────────────────┘
 │ JSON-RPC (HTTP) ↕
┌──────────────────────────────────────────────────┐
│ Rust core (`openhuman` binary, `src/`) │
│ • Memory Tree pipeline │
│ • Integration adapters + auto-fetch scheduler │
│ • Provider router (model routing) │
│ • TokenJuice compression │
│ • Native tools (search, fetch, fs, git, …) │
│ • Voice (STT in, TTS out, Meet agent) │
└──────────────────────────────────────────────────┘
 │
┌──────────────────────────────────────────────────┐
│ React frontend (app/src/) │
│ • Screens, navigation │
│ • Talks to core over `coreRpcClient` │
│ • No business logic - presentation only │
└──────────────────────────────────────────────────┘
```

**Where logic lives:**

* **Rust core**. all business logic. Memory Tree, integrations, model routing, tools, voice. Authoritative.
* **Tauri shell**. windowing, process lifecycle, IPC. A delivery vehicle, not where features live.
* **React frontend**. UI and orchestration. Calls into core via JSON-RPC.

## Data flow

1. **Connect**. OAuth into an [integration](../../features/integrations/README.md). Native providers store tokens in the core's encrypted `AuthService`; long-tail Composio providers live in the user's Composio tenant.
2. **Auto-fetch**. Every twenty minutes the [scheduler](../../features/obsidian-wiki/auto-fetch.md) walks every active connection and asks each native provider to sync.
3. **Canonicalize**. Provider output (an email page, a GitHub diff, a Slack channel dump) is normalized into provenance-tagged Markdown.
4. **Chunk**. Markdown is split into ≤3k-token deterministic chunks.
5. **Store**. Chunks land in SQLite (`<workspace>/memory_tree/chunks.db`) and as `.md` files in `<workspace>/wiki/`.
6. **Score**. Background workers run embeddings, entity extraction, hotness scoring.
7. **Summarize**. Source / topic / global summary trees are built and refreshed from the chunk pool.
8. **Retrieve**. When you ask a question, the agent queries the Memory Tree (search / drill down / topic / global / fetch).
9. **Compress**. Tool output and large source data go through [TokenJuice](../../features/token-compression.md) before entering LLM context.
10. **Route**. The [router](../../features/model-routing/) picks the right provider+model for the task hint.

## Privacy boundary

Stays in the active core workspace:

* The Memory Tree SQLite DB.
* The Obsidian Markdown vault.
* Audio capture buffers and any local model state.

Goes to configured external services:

* LLM calls to BYO providers or local runtimes.
* Web search via configured search provider.
* Composio direct-mode tool calls and webhooks.
* TTS/STT provider traffic.

See [Privacy & Security](../../features/privacy-and-security.md) for the full picture.

## Open source

* **Repo:** [github.com/tinyhumansai/openhuman](https://github.com/tinyhumansai/openhuman). GNU GPL3.
* **Issues and PRs** are welcome. The project is in early beta.
* For contributors, the canonical developer guide is [deep architecture reference](../architecture.md).
