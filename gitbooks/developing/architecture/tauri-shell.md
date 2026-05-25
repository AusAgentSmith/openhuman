---
description: The desktop host (`app/src-tauri/`) - Tauri v2 + WebView, IPC, local/remote core lifecycle, core bridge.
icon: desktop
---

# Tauri shell (`app/src-tauri/`)

The desktop host for OpenHuman: Tauri v2 + WebView, IPC commands, window management, and bridging to the active `openhuman-core` JSON-RPC endpoint. In Local runtime mode the core runs as an in-process tokio task inside the Tauri host. In Cloud runtime mode the host uses a user-configured remote URL + bearer token and does not spawn the embedded core. It does **not** duplicate the full domain stack; that lives in the repo-root Rust crate (`openhuman_core`, `src/main.rs`).

## Responsibilities

1. **Web UI**. Load the Vite build from `app/dist` (or dev server on port 1420).
2. **IPC**. Expose a small, explicit set of Tauri commands (see [Commands](#commands)).
3. **Core lifecycle**. Configure Local vs Cloud runtime, start the embedded core only in Local mode, and expose the active core URL/token to frontend services.
4. **AI prompts on disk**. Resolve bundled `src/openhuman/agent/prompts` from resources / dev cwd for `ai_get_config` / `write_ai_config_file`.
5. **Window + tray**. Desktop window behavior and system tray (see `lib.rs`).

## Core packaging

There is no Tauri sidecar binary in current builds. `pnpm core:stage` is a compatibility no-op; Local mode links the repo-root `openhuman` crate into the Tauri host and starts its JSON-RPC server in-process. The repo-root `openhuman-core` CLI binary still exists for Docker/server deployments and manual debugging.

## Stuck process recovery

Normal app quit runs teardown from `RunEvent::ExitRequested`: child webviews are closed before CEF shutdown, the embedded core's cancellation token is triggered, and the final process sweep sends `SIGTERM` to direct children before escalating holdouts with `SIGKILL` after a short grace period. Sweep summaries are logged as `[app] sweep: term=N kill=M total=K`; any nonzero `kill` count is a warning and means a child ignored graceful shutdown.

On macOS, hard exits (Force Quit, `SIGKILL`, renderer crash) can skip normal teardown. The next launch runs startup recovery before CEF cache preflight: it lists OpenHuman processes whose executable path belongs to the launching `.app/Contents`, skips the current process, sends `SIGTERM`, waits briefly, then `SIGKILL`s stragglers that still match the same pid+command. Logs use the `[startup-recovery]` prefix.

Startup recovery skips when `OPENHUMAN_CORE_REUSE_EXISTING=1` is set (so manual CLI-core reuse still works) and when the CEF `SingletonLock` is held by a live process (so the normal second-instance path can fail without killing the already-running app). The Tauri command `process_diagnostics_list_owned` returns the currently owned process list; the macOS implementation is bundle-scoped, Linux/Windows currently return empty.


## Tauri shell architecture (`app/src-tauri/`)

### Overview

The **`app/src-tauri`** crate (Rust package **`OpenHuman`**, binary **`OpenHuman`**) is a **desktop-only** host. It embeds the React UI, registers plugins (deep link, opener, OS, notifications, autostart, updater), manages the main window and tray, and connects frontend services to the active core JSON-RPC endpoint.

Non-desktop targets fail at compile time (`compile_error!` in `lib.rs`).

### Directory layout (actual)

```
app/src-tauri/src/
├── lib.rs                 # `run()`, tray/menu actions, plugins, `generate_handler!`, core startup
├── main.rs                # Binary entry
├── core_process.rs        # CoreProcessHandle, local embedded core lifecycle
├── core_rpc.rs            # Active core URL/token state + authenticated HTTP helpers
├── cdp/                   # Chrome DevTools Protocol helpers for CEF scanners
├── *_scanner/             # Provider scanner modules
└── webview_accounts/      # Third-party account windows and native event plumbing
```

There is **no** `src-tauri/src/services/session_service.rs` in this tree; session semantics are handled in the web layer + backend + core as applicable.

### Data flow: UI → core

```
React services/coreRpcClient
    → resolve URL/token from stored runtime preference or Tauri commands
        → HTTP POST <active-core-url>
            → Local embedded core or remote openhuman-core server
```

`BootCheckGate` drives runtime selection. For Cloud mode it stores the URL/token, calls `configure_core_rpc_connection`, and the Tauri host shuts down/skips the embedded core. For Local mode it clears remote credentials, calls `configure_core_rpc_connection` with no URL, then invokes `start_core_process`.

### Window and tray behavior

- The shell creates a tray icon at startup and wires actions to open the main window or quit.
- In daemon mode (`daemon` / `--daemon`), the main window is hidden on launch and can be reopened from tray actions.
- On macOS `RunEvent::Reopen` also restores and focuses the main window.
- Windows and Linux use the same tray actions (`Open OpenHuman`, `Quit`), with desktop-environment-specific tray rendering differences on some Linux setups.

### Bundled resources

`tauri.conf.json` bundles **`../../skills/skills`** and **`../../src/openhuman/agent/prompts`** so skills and prompt markdown ship with the app.

### Related

- IPC surface: see the [Commands](#tauri-ipc-commands-app-src-tauri) section below
- HTTP bridge: see the [Core bridge & helpers](#core-bridge-helpers-app-src-tauri) section below
- Rust domains (implementation): repo root `src/openhuman/`, `src/core_server/`


## Tauri IPC commands (`app/src-tauri`)

All commands are registered in **`app/src-tauri/src/lib.rs`** inside `tauri::generate_handler![...]` (desktop build). Names below are the **Rust** command names (camelCase in JS via serde where applicable).

### Demo / diagnostics

| Command | Purpose                                    |
| ------- | ------------------------------------------ |
| `greet` | Demo string (safe to remove in production) |

### AI configuration (bundled prompts)

| Command                | Purpose                                                                                      |
| ---------------------- | -------------------------------------------------------------------------------------------- |
| `ai_get_config`        | Build `AIPreview` from resolved `SOUL.md` / `TOOLS.md` under bundled or dev `src/openhuman/agent/prompts` |
| `ai_refresh_config`    | Same read path as `ai_get_config` (refresh hook)                                             |
| `write_ai_config_file` | Write a single `.md` under repo `src/openhuman/agent/prompts` (dev / safe filename checks)                |

### Core JSON-RPC commands

| Command | Purpose |
| --- | --- |
| `core_rpc_url` | Return the active core RPC URL. Local mode is loopback; Cloud mode is the configured remote `/rpc` URL. |
| `core_rpc_token` | Return the bearer token for the active core. Local mode uses the per-launch embedded token; Cloud mode uses the configured remote token. |
| `configure_core_rpc_connection` | Set Cloud mode URL/token or clear back to Local mode. Cloud mode stops/skips the embedded core. |
| `start_core_process` | Start the embedded local core. No-op when Cloud mode is configured. |
| `restart_core_process` | Restart the embedded local core. Returns an error in Cloud mode. |

Use **`app/src/services/coreRpcClient.ts`** (`callCoreRpc`) from the frontend.

### Window management

From **`commands/window.rs`** (names may vary slightly; see `lib.rs`):

| Command             | Purpose           |
| ------------------- | ----------------- |
| `show_window`       | Show main window  |
| `hide_window`       | Hide main window  |
| `toggle_window`     | Toggle visibility |
| `is_window_visible` | Query visibility  |
| `minimize_window`   | Minimize          |
| `maximize_window`   | Maximize          |
| `close_window`      | Close             |
| `set_window_title`  | Set title string  |

### OpenHuman daemon / service helpers

From **`commands/openhuman.rs`** (see source for exact payloads):

| Command                            | Purpose                                        |
| ---------------------------------- | ---------------------------------------------- |
| `openhuman_get_daemon_host_config` | Read daemon host preferences (e.g. tray)       |
| `openhuman_set_daemon_host_config` | Persist daemon host preferences                |
| `openhuman_service_install`        | Install background service (platform-specific) |
| `openhuman_service_start`          | Start service                                  |
| `openhuman_service_stop`           | Stop service                                   |
| `openhuman_service_status`         | Query status                                   |
| `openhuman_service_uninstall`      | Uninstall service                              |

### Screen share picker (CEF / macOS)

From **`screen_capture/mod.rs`**. Backs the in-page `getDisplayMedia` shim in `webview_accounts/runtime.js`. Session-gated: the shim must open a session with a live user gesture before enumeration / thumbnail captures succeed. See issue #713 (picker UX) + #812 (session gating).

| Command                           | Purpose                                                                                                                 |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `screen_share_begin_session`      | Open a 30s session from an account webview, after a `navigator.userActivation.isActive` gesture. Returns `{ token, sources }`. Rate-limited to 10/minute per account. |
| `screen_share_thumbnail`          | Capture a single source's thumbnail as base64 PNG. Requires a live token and an `id` that the session was issued for. macOS only; other platforms return an error.    |
| `screen_share_finalize_session`   | Close the session. Called by the shim on Share or Cancel; safe to call with an unknown/expired token (no-op).                                                         |

### Removed / not present

The following **do not** exist in the current `generate_handler!` list: `core_rpc_relay`, `exchange_token`, `get_auth_state`, `socket_connect`, `start_telegram_login`. Authentication, sockets, and product JSON-RPC are handled in the **React** services layer and the **core** process, not via these IPC names.

### Example: core RPC

```typescript
import { callCoreRpc } from "../../services/coreRpcClient";

const result = await callCoreRpc({
  method: "your.rpc.method",
  params: { foo: "bar" },
});
```

---

_See `app/src-tauri/src/lib.rs` for the authoritative list._


## Core bridge & helpers (`app/src-tauri`)

This document replaces the old “SessionService / SocketService” split. The Tauri crate **does not** embed a duplicate Socket.io server or Telegram client; instead it focuses on window/process management and authenticated HTTP JSON-RPC to the active core endpoint.

### `CoreProcessHandle` (`core_process.rs`)

- Starts the embedded core JSON-RPC server in Local mode.
- Skips startup when Cloud mode is configured.
- Used during app setup in `lib.rs` (`app.manage(core_handle)`).

### `core_rpc` (`core_rpc.rs`)

- Owns active core connection state: Local vs Cloud.
- Normalizes server-origin URLs to `/rpc`.
- Applies `Authorization: Bearer <token>` for Tauri-side HTTP calls, including scanner/native event paths.

### `commands/openhuman.rs`

- Daemon host JSON config (e.g. tray visibility) under the app data directory.
- Install/start/stop/status/uninstall helpers for the **openhuman** background service.

### `utils/dev_paths.rs`

- Resolves **`src/openhuman/agent/prompts`** for development and bundled resource paths for AI preview.

### `utils/tauriSocket.ts` (frontend)

Not in `src-tauri`, but **pairs** with the shell: the React app listens for Tauri events that mirror socket activity when using the Rust-side client. See `app/src/utils/tauriSocket.ts` and the [Frontend Services](frontend.md#services-layer) chapter.

---
