---
description: >-
  Install OpenHuman, choose Local or Cloud runtime, configure AI and optional
  integrations, and run your first request against your own Memory Tree.
icon: play
---

# Getting Started

This page walks you through installing OpenHuman, choosing a runtime, configuring AI, and running your first request.

OpenHuman is open source under the GNU GPL3 license. The codebase is at [github.com/tinyhumansai/openhuman](https://github.com/tinyhumansai/openhuman).

***

## System requirements

OpenHuman runs on **macOS, Windows and Linux** desktops. 4 GB+ RAM is recommended; 16 GB+ if you intend to ingest very large mailboxes or repos, or run a [local model](../features/model-routing/local-ai.md) on the same machine.

### Permissions

The first time you launch OpenHuman, the OS will prompt for the permissions the app needs (Accessibility on macOS, Input Monitoring for the voice hotkey, Camera/Microphone if you plan to use the [Meeting Agent](../features/mascot/meeting-agents.md)). You can review and adjust these any time under **Settings → Automation & Channels**.

***

## 1. Download and install

Get the OpenHuman desktop app from [http://tinyhumans.ai/openhuman](http://tinyhumans.ai/openhuman) or via your platform's package manager. Open the app once it's installed.

## 2. Choose a runtime

The first screen asks whether to run the core locally or connect to a remote core:

* **Local** starts `openhuman-core` inside the desktop app. Use this for one-machine/offline-first setups.
* **Cloud** connects the desktop app to a self-hosted `openhuman-core` server. Paste the server URL and bearer token from your deployment.

There is no OpenHuman account login in the closedhuman fork.

## 3. Configure AI

Open **Settings → AI** and add your provider key. The default path is a BYO OpenAI-compatible provider, but Anthropic, OpenRouter, local Ollama, LM Studio, vLLM, and other compatible endpoints can be configured too.

## 4. Optional: connect integrations

For native Google/GitHub coverage, use the built-in OAuth flow or the `oauth-connect` CLI. Gmail, Calendar, Drive, and GitHub have direct Rust providers for selected actions.

For long-tail integrations such as Discord, Slack, Notion, Jira, and non-native Gmail actions, first open **Settings → Developer Options → Composio Routing (Direct Mode)** and paste your Composio API key. Then connect services from the Skills/Integrations grid. OpenHuman creates missing Composio managed auth configs automatically before opening the hosted OAuth page.

Real-time integration triggers require **Settings → Developer Options → Composio Triggers (Direct Mode)** with an ngrok static domain and authtoken.

## 5. Run your first request

Once chat is responding, try prompts like:

**Briefings**

* "What do I need to know from the last 12 hours?"
* "What's waiting on me?"

**Cross-source queries**

* "Summarize what I missed today."
* "What are the key decisions from this week?"
* "Extract action items from my recent conversations."
* "What did Sarah say about the project across email and chat?"

OpenHuman picks the right model for each task automatically. See [Automatic Model Routing](../features/model-routing/).

***

## 6. Open the Obsidian vault

The Memory tab has a **View vault in Obsidian** button. Click it to open `<workspace>/wiki/` in [Obsidian](https://obsidian.md). You can browse the agent's summaries, drop in your own notes, and even build manual links - the agent will pick up your edits on the next ingest. See [Obsidian-Style Memory](../features/obsidian-wiki/).

***

## 7. Let the mascot do more

Now that the agent has memory and a model, the rest of the product is about giving it more surfaces:

* [**Meeting Agents**](../features/mascot/meeting-agents.md) - drop a Google Meet link in and the mascot joins as a real participant: it listens, takes notes into the Memory Tree, speaks back into the call, and uses tools live.
* [**Auto-fetch from Integrations**](../features/obsidian-wiki/auto-fetch.md) - connect more sources from **Settings**; every twenty minutes the scheduler pulls fresh data into your tree.
* [**Native Voice**](../features/native-tools/voice.md) - push-to-talk dictation and TTS replies so you can talk to OpenHuman instead of typing.
* [**Subconscious Loop**](../features/subconscious.md) - let the mascot keep working on standing tasks while you're away.

## Join the community

OpenHuman is in early beta. Feedback and contributions make a real difference at this stage.

* **GitHub:** [github.com/tinyhumansai/openhuman](https://github.com/tinyhumansai/openhuman)
* **Discord:** [discord.tinyhumans.ai](https://discord.tinyhumans.ai)
