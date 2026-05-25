---
description: The agent's view of the 118+ connected third-party services.
icon: plug
---

# Third-party Integrations

OpenHuman's agent can call into [118+ third-party services](../integrations/README.md) - Gmail, Notion, GitHub, Slack, Stripe, Calendar, and the long tail - through native providers plus Composio direct mode.

## How it shows up to the agent

Once you've connected a service via OAuth, its actions become callable tools. The agent doesn't need to know whether a tool talks to Gmail, GitHub, Discord, or a local file - it just calls the tool. Covered Gmail / Calendar / Drive / GitHub slugs route to native Rust clients; long-tail toolkits route through your personal Composio tenant.

A few examples of what becomes available:

* "Send a message to #engineering on Slack."
* "Create an issue in the openhuman repo."
* "What's on my calendar tomorrow?"
* "Pull the last 20 Stripe charges over $1000."

## Native vs proxied

Some services have **native providers** - Rust modules that know how to ingest the service into the [Memory Tree](../obsidian-wiki/memory-tree.md) directly and execute selected actions with provider OAuth tokens stored in the core. Others are **Composio-managed tools**: the agent can call them through your Composio API key, but there's no automatic ingest yet. New native providers are added as features land.

## Privacy boundary

There is no OpenHuman integration backend in this fork. Native provider tokens and the Composio API key are stored by the active core's encrypted credential store. Composio-managed provider tokens live in your Composio tenant. The agent only sees the *results* of tool calls, not the credentials.

## See also

* [Third-party Integrations (catalog)](../integrations/README.md) - the user-facing pitch, OAuth flow, and connection management.
* [Auto-fetch](../obsidian-wiki/auto-fetch.md) - how connected services flow into the Memory Tree.
* [Privacy & Security](../privacy-and-security.md) - the full boundary.
