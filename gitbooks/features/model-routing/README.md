---
description: >-
  BYO providers, many models. Tasks pick their model via workload routing:
  reasoning goes to a strong model, fast paths go to a fast one, vision to vision.
icon: route
---

# Automatic Model Routing

Different parts of an agent want different models. Long reasoning wants a frontier model. Quick "fix this typo" calls want a fast cheap one. Vision wants a vision model. OpenHuman handles this with workload routing so you configure providers once and let tasks pick the right route.

## How a request gets routed

The model parameter on any chat call can take one of two shapes:

- **Concrete provider/model name**. e.g. `openai:gpt-5.4` or `anthropic:claude-sonnet-4`. Routes through the provider factory with that exact model.
- **Hint prefix**. e.g. `hint:reasoning`. Looks the hint up in the route table and resolves to a `(provider, model)` pair.

```toml
[local_ai]
chat_provider = "openai:gpt-5.4"
reasoning_provider = "openai:gpt-5.4"
coding_provider = "anthropic:claude-sonnet-4"
memory_provider = "openai:gpt-5.4-mini"

[[cloud_providers]]
slug = "openai"
endpoint = "https://api.openai.com/v1"
auth_style = "Bearer"
```

The provider factory resolves workload routes from `config.toml` and the cloud-provider rows configured in Settings. Routes can be changed without depending on an OpenHuman-hosted backend.

## Common hints

| Hint | Typical target | When it's used |
| --- | --- | --- |
| `hint:reasoning` | A strong reasoning model | Multi-step planning, math, code-heavy turns |
| `hint:fast` | A fast/cheap model | UI helpers, autocompletes, small classification calls |
| `hint:vision` | A vision-capable model | Screenshots, image attachments, OCR |
| `hint:summarize` | A model good at compression | Memory tree summary builders |
| `hint:code` | A code-tuned model | Native coder turns |

The exact mappings are configurable; the defaults ship sensible per-provider routes.

## BYO providers

Routing in this fork uses the providers you configure under **Settings → AI**. There is no OpenHuman subscription or model proxy. Add OpenAI, Anthropic, OpenRouter, or an OpenAI-compatible local/runtime endpoint, then set workload routes such as `chat_provider`, `reasoning_provider`, `coding_provider`, and `memory_provider` to `<slug>:<model>` strings.

## Overriding routes

- **Globally**. config TOML (`Config` struct in `src/openhuman/config/schema/types.rs`) can supply a custom route table at startup.
- **Per call**. pass a concrete model name (no `hint:` prefix) and the router falls through to the default provider with that exact model.
- **For a skill**. skills can pin a hint or a model in their manifest.

## Why this isn't just "model switcher"

Routing isn't a UI dropdown. The agent loop itself emits hints based on what it's about to do. You don't pick the model; the *task* does. That's the difference between "multi-model" and "smart routing".

## See also

- [Smart Token Compression](../token-compression.md). what makes large reasoning calls affordable.
- [Native Tools](../native-tools/README.md). different tool calls hint at different routes.
- [Local AI (optional)](local-ai.md). lightweight chat hints can run on-device.
