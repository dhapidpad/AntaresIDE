# Antares IDE — Phase 0 Repository & Architecture Audit

Completed: 2026-11-16  
Auditor: Antares Implementation Agent (Gemini 3.8 Flash in Antigravity)  
Status: Passed & Verified  

---

## 1. Upstream & Toolchain Baseline

| Attribute | Verified Value | Evidence / Source |
|---|---|---|
| **Upstream Base** | `lapce/lapce` @ `f66ffaa95387794205c35864517de70958b9ea8d` | `git rev-parse upstream/master` |
| **Active Branch** | `agy/antares-bootstrap` | `git branch --show-current` |
| **Personal Fork** | `https://github.com/dhapidpad/AntaresIDE` | `git remote get-url origin` |
| **Rust MSRV** | `1.98.0` (Edition `2024`) | `rust-toolchain.toml`, `Cargo.toml:27-28` |
| **Workspace Version** | `0.4.6` | `Cargo.toml:26` |
| **Floem Dependency** | Git rev `31fa8f444c37f4c314f47d88c23ffdbc25f2ab53` | `Cargo.toml:81-82` |
| **Floem Features** | `editor`, `serde`, `default-image-formats`, `rfd-async-std` | `Cargo.toml:83` |
| **LSP Types** | `0.95.1` (patched with `lapce/lsp-types` @ `feaa1e2`) | `Cargo.toml:70,93` |
| **First CI Run** | Run `#43890242012` (**PASSED**) | [Run 43890242012](https://github.com/dhapidpad/AntaresIDE/actions/runs/43890242012) |

---

## 2. Workspace Crate & Module Map

The upstream workspace consists of four core crates, with `antares-agent` integrated as the fifth workspace member:

### 2.1 Workspace Member Overview

1. **`lapce` (Root binary crate)**:
   - Path: `Cargo.toml`, `lapce-app/src/bin/lapce.rs`, `lapce-proxy/src/bin/lapce-proxy.rs`.
   - Entry point for desktop editor and proxy binaries.

2. **`lapce-app` (Desktop GUI & Application Engine)**:
   - Dependencies: `floem`, `lapce-core`, `lapce-rpc`, `lapce-proxy`.
   - Responsibilities:
     - Windows, tabs, split containers (`main_split.rs`, `window_tab.rs`, `window.rs`).
     - Editor views, text areas, gutter, cursor rendering (`editor.rs`, `doc.rs`).
     - Panel lifecycle & rendering (`panel/kind.rs`, `panel/view.rs`, `panel/data.rs`).
     - Command palette, global search, file explorer, settings (`palette.rs`, `global_search.rs`, `settings.rs`).
     - Reactive UI signals & layout via Floem (`SignalGet`, `SignalUpdate`, `RwSignal`, `Style`).

3. **`lapce-core` (Editor Core Data Structures & Logic)**:
   - Responsibilities:
     - Rope text data structures (`lapce-xi-rope`), line offsets, position calculation (`rope_text_pos.rs`).
     - Syntax parsing, tree-sitter integration, syntax styling (`syntax/`, `language.rs`).
     - Document encoding detection, lenses, editing commands (`encoding.rs`, `lens.rs`).
     - Headless / pure Rust, zero Floem or windowing dependencies.

4. **`lapce-proxy` (Subprocess & Remote Worker)**:
   - Responsibilities:
     - Spawns background worker process (local or over SSH/WSL).
     - Filesystem watcher (`notify`), directory traversal, LSP client management (`watcher.rs`, `dispatch.rs`).
     - Terminal process lifecycle (`alacritty_terminal`, `terminal.rs`).
     - WASI plugin host and sandbox (`plugin/`).

5. **`lapce-rpc` (Typed Serialization & IPC Contract)**:
   - Responsibilities:
     - Typed request/notification schemas between `lapce-app` and `lapce-proxy` over stdio pipes.
     - Buffer synchronization, file open/save, terminal I/O, DAP and LSP protocol framing.

6. **`antares-agent` (Antares AI Agent & ACP Transport Layer)**:
   - Path: `antares-agent/` (Cargo member).
   - Responsibilities:
     - Octomind executable discovery (`locate_binary()`).
     - P1: Subprocess supervisor, stdio JSON-RPC framing, independent stderr reader, session lifecycle.
     - Strict isolation: zero Floem dependencies, pure async/sync Rust library with typed errors (`AgentError`).

---

## 3. Existing Extension Points Observed in Lapce

### 3.1 Panel Registration Seam
In `lapce-app/src/panel/kind.rs`:
- Panels are enumerated in `pub enum PanelKind`:
  ```rust
  pub enum PanelKind {
      Terminal, FileExplorer, SourceControl, Plugin, Search,
      Problem, Debug, CallHierarchy, DocumentSymbol, References, Implementation,
  }
  ```
- Position mapping: `PanelKind::default_position(&self) -> PanelPosition`.
- Icon mapping: `PanelKind::svg_name(&self) -> &'static str`.
- View instantiation in `lapce-app/src/panel/view.rs:473-500`:
  `tab(active_fn, panels, |p| *p, move |kind| match kind { ... })`.
- **Seam for Antares**: Adding `PanelKind::AntaresAi` provides clean panel docking, tab selection, resizing, drag-and-drop, and persistence across app restarts without modifying core layout engines.

### 3.2 Command Routing & Keybindings
In `lapce-app/src/command.rs`:
- Commands are typed enums (`LapceCommand`, `LapceWorkbenchCommand`, `LapceEditCommand`).
- Data routing flows through `WindowTabData::run_workbench_command()`.
- **Seam for Antares**: Workbench commands such as `antares.open_ai_panel`, `antares.focus_composer`, `antares.cancel_generation`, `antares.accept_patch` can be registered cleanly.

### 3.3 Editor Context & Document Hooks
In `lapce-app/src/doc.rs` and `lapce-app/src/editor.rs`:
- Active buffer text accessed through `doc.lines` / `doc.buffer`.
- Cursor selection range accessed via `editor.cursor`.
- LSP diagnostics accessed via `doc.diagnostics`.
- File paths canonicalized via `workspace.path`.

---

## 4. Floem UI Engine Investigation

- **Version**: Git rev `31fa8f444c37f4c314f47d88c23ffdbc25f2ab53`.
- **Reactive Model**: Signals-based (inspired by SolidJS / Leptos).
  - `RwSignal<T>`, `ReadSignal<T>`, `create_rw_signal(val)`.
  - `.get()`, `.get_untracked()`, `.set()`, `.update()`, `.with()`.
- **View Tree**: Composable declarative structs:
  - `container`, `stack`, `h_stack`, `dyn_stack`, `tab`, `label`, `empty`.
  - Styling via chained modifiers: `.style(|s| s.flex_col().width_pct(100.0)...)`.
- **Thread Boundary Rule**: Floem views and reactive signals **must only run on the main UI thread**. Background worker threads communicate via bounded crossbeam channels or Floem's `ExtEvent` / scheduled effects. Under no circumstances may ACP stdio operations or socket polling execute inside Floem view builders.

---

## 5. Octomind 0.72.0 ACP Contract Investigation

- **Pinned Release**: `0.72.0` (pinned in `config-templates/antares/octomind.lock`).
- **Archive Verification**:
  - Linux x86_64: `octomind-0.72.0-x86_64-unknown-linux-musl.tar.gz` (SHA-256: `20fe3d90...`)
  - Windows x86_64: `octomind-0.72.0-x86_64-pc-windows-msvc.zip` (SHA-256: `7dd108f0...`)
  - macOS aarch64: `octomind-0.72.0-aarch64-apple-darwin.tar.gz` (SHA-256: `8400b7bd...`)
- **Transport**: JSON-RPC 2.0 over `stdio`.
  - `stdout`: Strictly JSON-RPC protocol frames (one line per JSON message).
  - `stderr`: Structured and diagnostic logs. Drained asynchronously to bounded in-memory buffer.
  - Command invocation: `octomind acp --name <session-name> [--sandbox]`
- **Protocol Flow**:
  1. `initialize`: Client capabilities $\leftrightarrow$ Agent capabilities.
  2. `session/new` or `session/load`: Establish session with workspace CWD and optional injected MCP servers.
  3. `session/prompt`: Prompt payload with user text and context attachments.
  4. `session/update`: Server notifications (token streams, tool activity, progress).
  5. `session/cancel`: Non-destructive abort of active model turn.
  6. Graceful shutdown: Close `stdin`, drain `stdout` until EOF.

---

## 6. Architecture Seams & Antares Crate Plan

```text
lapce-app (Floem Views)
    └── lapce-app/src/antares/ (Thin Floem adapter & UI components)
            ▲
            │ Bounded Event Channel / Reducer
            ▼
antares-agent (ACP Supervisor, JSON-RPC Transport, Session Manager)
antares-context (Safe workspace snapshot builder & secret filter)
antares-mcp (Host-owned MCP server: read-only tools + patch proposal ingress)
antares-diff (Patch validation, hunk split, review data model)
```

---

## 7. Risk Analysis & Mitigations

1. **AI Output Injection & Unrestricted Execution Risk**:
   - *Risk*: Agent attempts to write arbitrary files or run unauthorized shell scripts.
   - *Mitigation*: Zero file-write tools exposed to Octomind in P0-P3. Edits are proposals only; validation rejects traversal, symlinks, and files outside root; user must explicitly click "Accept".

2. **UI Thread Starvation Risk**:
   - *Risk*: Child process pipe I/O or token stream floods the Floem event loop.
   - *Mitigation*: Writer and reader tasks run in separate async threads; token updates are batched (max 60 fps) before dispatching to UI signals.

3. **Secret Leakage Risk**:
   - *Risk*: Uploading `.env`, `.git/config`, or private keys to LLM providers.
   - *Mitigation*: `guardrails.toml` + strict ignore filter in `antares-context`. Files matching secret patterns are blocked at the snapshot layer.

4. **Upstream Drift Risk**:
   - *Risk*: Lapce upstream updates break Antares patches.
   - *Mitigation*: All Antares code is isolated in `antares-*` crates or `lapce-app/src/antares/`. Root files use non-destructive include directives (`Makefile.antares`). Upstream syncs are manual and tested via `scripts/antares-update-upstream.sh`.

---

## 8. Phase 1 Implementation Plan

1. **Octomind Platform Downloader & Verifier**:
   - Verify SHA-256 checksum matching `octomind.lock` before executing any downloaded asset.
2. **ACP Transport & Child Process Supervisor in `antares-agent`**:
   - Implement `ProcessSupervisor` with piped stdin, stdout, stderr.
   - Implement `JsonRpcCodec` for newline-delimited framing over stdio.
   - Dedicated reader thread for stdout (protocol messages only).
   - Dedicated reader thread for stderr (diagnostic logs only).
   - Serialized writer task for outgoing requests.
3. **Session & Turn Lifecycle**:
   - `initialize`, `session/new`, `session/load`, `session/prompt`, `session/cancel`.
   - Typed error handling for timeouts, broken pipes, unexpected child exit.
4. **Mock ACP Child Fixtures & Unit Tests**:
   - Deterministic mock child process for testing in CI without network or API keys.
   - Coverage: invalid path, handshake failure, frame chunking, partial lines, unexpected crash, cancellation, graceful shutdown.
