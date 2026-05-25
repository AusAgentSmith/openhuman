---
icon: shield
---

# Privacy & Security

OpenHuman is designed so that the **memory of your life lives in the active core workspace you control**. In Local mode that is your desktop workspace. In Cloud mode that is your self-hosted `openhuman-core` server. The closedhuman fork does not use an OpenHuman-hosted product backend for login, LLM calls, OAuth, billing, or integration proxying.

***

## Privacy by Design

**The Memory Tree is workspace-local.** The SQLite database (`<workspace>/memory_tree/chunks.db`) and the Markdown vault (`<workspace>/wiki/`) live in the active core workspace. The agent reads from that workspace directly; nothing about your raw source data sits on an OpenHuman product backend.

**Integration credentials stay out of plaintext config.** Native OAuth tokens and BYO API keys are stored by the core's encrypted `AuthService`, not in `config.toml`. Long-tail Composio provider tokens live in your personal Composio tenant.

**Encrypted credential storage.** Sensitive tokens are stored in the core's encrypted `AuthService` credential store. In Cloud mode that store is on the server workspace; in Local mode it is on the desktop workspace.

**No training on your data.** Your conversations, your Memory Tree, and your personal information are never used to train AI models or improve systems.

**Optional** [**Local AI**](model-routing/local-ai.md)**.** If you want embeddings and summary-tree building to stay on your machine, opt in. Heartbeat / learning / subconscious loops can be moved on-device the same way.

***

## What stays in your workspace

|                                 |                                                                 |
| ------------------------------- | --------------------------------------------------------------- |
| **Memory Tree SQLite database** | `<workspace>/memory_tree/chunks.db` on the active core. |
| **Obsidian Markdown vault**     | `<workspace>/wiki/`. Yours to read, edit, copy, delete. |
| **Audio capture buffers**       | Local to the device doing capture; discarded after STT. |
| **Local model state**           | Local to the configured model runtime. |

## What external services handle

|                                    |                                                                                                                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **LLM calls**                      | Sent directly from the active core to the user's configured provider (OpenAI / Anthropic / OpenRouter / local OpenAI-compatible runtime) per the [model router](model-routing/). |
| **Web search**                     | Uses whatever search provider credentials or local configuration the core is given; there is no OpenHuman search proxy in this fork. |
| **Integration OAuth & tools**      | Native Google/GitHub coverage uses tokens in `AuthService`; long-tail integrations use the user's Composio tenant and API key. |
| **TTS**                            | Preferred path is local/OpenAI-compatible Kokoro; hosted backend ElevenLabs proxy is effectively dead in this fork. |

***

## Permissions and access control

OpenHuman accesses an integration only after you complete its OAuth flow. Each connection has its own scope; you can revoke any of them at any time from the Skills tab.

[Auto-fetch](obsidian-wiki/auto-fetch.md) does run continuously while a connection is active, that is the whole point. But it is bound by:

* The **OAuth scope** you granted that integration.
* A **per-provider sync interval** (e.g. Gmail every 15 min by default).
* A **daily budget** per connection that caps API usage.

If you revoke a connection, the next tick stops syncing it; chunks already in your local Memory Tree remain there because they're yours.

***

## Why a local memory is privacy

Most AI assistants face a tradeoff: more context means more raw data sent to the cloud. The Memory Tree eliminates this tradeoff.

Because canonicalization, chunking, scoring and summary trees all run **inside your local Rust core**, your raw source data never leaves your machine. The only thing the LLM sees is what the agent retrieves from your local Memory Tree at the moment of a turn, and that retrieval is governed by your prompt, not by background uploads.

Compression and locality together become the privacy architecture.

<figure><img src="../.gitbook/assets/V17 — Privacy Shield@2x.png" alt=""><figcaption></figcaption></figure>

## Security

**Encrypted in transit.** Cloud-mode desktop clients should connect to the self-hosted core over TLS or a trusted private network. Provider traffic uses each provider's HTTPS endpoint.

**Sandboxed skills.** Each skill runs in its own isolated execution environment with enforced memory and resource limits. Skills cannot access each other's data, the host system's file system, or your credentials.

**Workspace-scoped tools.** The native [filesystem tools](native-tools/coder.md) operate within the workspace the user opens; they do not have ambient access to the rest of the disk.

**Bearer tokens.** Cloud-mode clients authenticate to the self-hosted core with bearer tokens. Treat `core.token` and any device-scoped client token as secrets.

***

## Trust & Risk Intelligence

OpenHuman includes an intelligence layer designed to help you reason about credibility, information quality, and potential risks across your connected sources.

**Scam and impersonation signals.** Behavioral patterns associated with scams, impersonation, or coordinated abuse can surface as warnings. Signals come from patterns, not from sharing individual message content.

**Contextual dynamic trust.** Trust is contextual, credibility in one domain does not automatically transfer to another. OpenHuman represents trust through aggregated artifacts and historical accuracy rather than static scores.

**Advisory, not enforcement.** Trust and risk outputs are advisory signals to inform your judgment. OpenHuman does not ban users, remove messages, or enforce moderation decisions.

***

## Shared environments

In team or community settings, privacy remains user-centric. Each user's connected sources are scoped to their account; admins do not get a backdoor into other users' Memory Trees.

Community-level intelligence is derived from aggregated and anonymized signals, never from direct access to individual message content.
