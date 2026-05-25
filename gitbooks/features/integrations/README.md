---
description: >-
  118+ third-party integrations - Gmail, Notion, GitHub, Slack, Stripe, Calendar
  and more - with native providers plus BYO Composio direct mode.
icon: plug
---

# Third-party Integrations (118+)

OpenHuman ships native providers for the core Google/GitHub path and uses **Composio direct mode** for the long-tail integration catalog. There is no OpenHuman-hosted integration proxy in this fork: you bring your own Composio API key, and the core calls Composio v3 from the active runtime.

Configure the key once under **Settings → Developer Options → Composio Routing (Direct Mode)**. In Cloud mode the key is stored in the server workspace's encrypted credential store; in Local mode it is stored in the local workspace. It is not stored in `config.toml`.

Once a service is connected, it shows up in four places at once:

1. As an **agent tool**, the model can call it directly.
2. As a **memory source**, [auto-fetch](../obsidian-wiki/auto-fetch.md) syncs it into the [Memory Tree](../obsidian-wiki/memory-tree.md) every twenty minutes.
3. As a **profile signal**, your activity across services feeds your personalization.
4. As a **trigger source**, live events (a new email, a new charge, an inbound DM) flow into the [Triggers](triggers.md) pipeline and can fire off agent actions automatically.

## Some of what's in the catalog

The catalog spans productivity, business, social, messaging and Google. A non-exhaustive sample:

| Category                | Examples                                             |
| ----------------------- | ---------------------------------------------------- |
| **Email & calendar**    | Gmail, Outlook, Google Calendar, Apple Calendar      |
| **Docs & storage**      | Google Docs, Google Drive, Notion, Dropbox, Airtable |
| **Code & dev**          | GitHub, Linear, Jira, Figma                          |
| **Comms**               | Slack, Discord, Microsoft Teams, Telegram, WhatsApp  |
| **CRM & sales**         | Salesforce, HubSpot                                  |
| **Commerce & payments** | Stripe, Shopify                                      |
| **Project management**  | Asana, Trello                                        |
| **Social**              | Twitter / X, Spotify, YouTube                        |

## Native vs Composio-managed

Some services have **native providers**: Rust modules that know how to ingest the service into the Memory Tree directly and execute selected actions without Composio (for example Gmail, Calendar, Drive, and GitHub). Other services are **Composio-managed**: the agent can call them through your Composio tenant, but there may not be an automatic ingest path yet. New native providers are added as features land.

## How connections work

Click **Connect** on any integration. The core calls `openhuman.composio_authorize`, looks up a Composio v3 auth config for that toolkit, creates a managed auth config if your Composio tenant does not have one yet, and opens Composio's hosted OAuth URL in the browser. Once you sign in, the connection becomes active and OpenHuman can use the toolkit through the agent tool surface.

For common managed-auth toolkits such as Gmail or Discord, you should not need to create an auth config manually in the Composio dashboard. If you see an error that says no auth config exists, you are likely running an older core build.

Each integration shows its current status:

* **Not connected**. integration has not been set up.
* **Connected**. integration is active and being synced.
* **Manage**. active integration with options to reconfigure or disconnect.

You can revoke any connection at any time from the Skills tab. The Composio API key itself is managed from **Settings → Developer Options → Composio Routing (Direct Mode)**.

## Triggers

Real-time integration events require a public webhook URL. Direct mode provides this through the embedded webhook receiver plus ngrok:

1. Open **Settings → Developer Options → Composio Triggers (Direct Mode)**.
2. Paste a static `*.ngrok-free.dev` domain and ngrok authtoken.
3. Enable the local receiver.

The receiver verifies Composio's HMAC signature, parses the v3 envelope, and publishes `DomainEvent::ComposioTriggerReceived` to the local event bus. See [Triggers](triggers.md) for the full pipeline.

## Messaging channels

Three integrations are special. OpenHuman uses them to _talk back_ to you, not just read from them:

* **Telegram**. the primary messaging channel. Two-way: send and receive messages, manage chats, search history, create groups, 80+ actions on your behalf. All actions run through your own encrypted credentials.
* **Discord**. send and receive messages via Discord. Connect your account to receive OpenHuman messages there.
* **Web**. a browser-based chat interface within the desktop app. Messages stay entirely local.

Set your default under **Settings → Automation & Channels → Messaging Channels**. The active route status shows which channel is currently in use. Telegram offers two credential modes: connect via OpenHuman (one-click, encrypted) or provide your own credentials for maximum control.

## Skills

Beyond third-party services, OpenHuman has **skills**, small sandboxed modules that run inside the app, fetch external data, run on a schedule, transform information, and respond to events. Each runs with enforced resource limits. Skills install from the Skills tab and integrate with the same Memory Tree as everything else.

## Native voice and tools

Two capabilities ship native rather than as integrations because they're load-bearing for the desktop experience:

* [**Voice**](../native-tools/voice.md). STT in, TTS out, plus a live Google Meet agent that joins meetings, transcribes them into your Memory Tree, and can speak back into the call.
* [**Native tools**](../native-tools/README.md). built-in web search, web-fetch scraper, and a full filesystem/git/lint/test/grep coder toolset that the agent uses out of the box.

## Privacy boundary

There is no OpenHuman integration backend in this fork. Integration traffic follows one of two paths:

* Native Google/GitHub coverage uses provider OAuth tokens stored by the core's encrypted `AuthService`.
* Long-tail toolkits go through your personal Composio tenant using the Composio API key you configured.

The agent sees tool results, not credentials. Secrets live in the active core workspace credential store, and Composio provider tokens live in your Composio tenant.

See [Privacy & Security](../privacy-and-security.md) for the full boundary.

## See also

* [Triggers](triggers.md), live events from connected integrations and how they fire agent actions.
* [Auto-fetch from Integrations](../obsidian-wiki/auto-fetch.md)
* [Memory Tree](../obsidian-wiki/memory-tree.md)
