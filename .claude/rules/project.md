# mcp-for-azure-devops-boards — Project Rules

## MANDATORY: Read These First

You MUST ALWAYS read these documents before ANY work, in this order:

1. **`docs/PROJECT.md`** — source of truth: tech stack, dependencies, configuration, transport modes, code-generation pipeline, Azure DevOps API integration, output optimization, build/lint/test commands, CI/CD, Docker, conventions. **Entry-level read.**
2. **`docs/ARCHITECTURE.md`** — runtime architecture: system/layer diagrams, project structure, request flow, MCP tool inventory, key data types, shared-state model. **Entry-level read.**

`PROJECT.md` and `ARCHITECTURE.md` are the canonical technical references. When in doubt, those docs win. Cross-reference rather than duplicating across documents or into these rules.

The language rules live in `rust.md` (Rust language, idioms, error handling, testing, Definition of Done). The MCP + Azure DevOps tooling rules (tool conventions, anti-prompt-injection, the `AzureDevOpsApi` trait, code generation, transport, API integration) live in `mcp.md`. The behavioral/process/git/plan rules live in `agent.md`. This file holds project-specific operational facts only.

---

## Project Overview

**mcp-for-azure-devops-boards** is a **Model Context Protocol (MCP) server** that exposes **Azure DevOps Boards and Work Items** operations as MCP tools consumable by LLM agents. It is a Rust (edition 2024) binary + library, built as a two-crate Cargo workspace, with a companion proc-macro crate (`mcp-tools-codegen`) for tool registration.

- It exposes **24 MCP tools** across 8 categories (organizations, projects, tags, work item types, classification nodes, teams, boards, work items), all named with the **`azdo_`** prefix.
- It runs over **two transports**: **stdio** (default) and **streamable HTTP** (`--server`, binds `0.0.0.0:<port>`).
- It authenticates to the Azure DevOps REST API (v7.1) via **`DefaultAzureCredential`** — Bearer tokens fetched per-request, never persisted, never logged.
- It can self-register into MCP clients via the **`--install`** flag (claude-code, claude-desktop, cursor, vscode, codex, gemini-cli).

---

## Repository Status — Public / Open Source

This is a **PUBLIC, open-source repository** — Rust MCP server, **MIT-licensed**, hosted on **GitHub** (`github.com/danielealbano/azure-devops-boards-mcp-rust`; crate metadata declares `mcp-for-azure-devops-boards`). CI and releases run on **GitHub Actions / GitHub Releases**.

- The project is distributed publicly; keep code, docs, and configuration consistent with an open-source, MIT-licensed posture.
- Do NOT introduce private/internal or proprietary framing.
- Do NOT hardcode organization-specific identifiers, credentials, endpoints, or secrets — all Azure context is resolved at runtime from CLI flags and `DefaultAzureCredential`.

---

## Commit Scope — ABSOLUTE RULE

Per `agent.md`, every commit uses `<type>(<scope>): <short description>`. Scopes for this repo map to the source layout in `docs/ARCHITECTURE.md` (Project Structure):

| Scope | Applies to |
|---|---|
| `azure` | `src/azure/` (`client.rs`, `api_trait.rs`, `models.rs`, and the per-area API modules: `boards.rs`, `classification_nodes.rs`, `iterations.rs`, `organizations.rs`, `projects.rs`, `tags.rs`, `teams.rs`, `work_items.rs`) |
| `mcp` | `src/mcp/` (`server.rs`, `AzureMcpServer`, `ServerHandler`) |
| `tools` | `src/mcp/tools/` (per-tool implementations and `support/` shared utilities) |
| `server` | `src/server/` (`http.rs` — hyper + rmcp `StreamableHttpService`) |
| `compact` | `src/compact_llm.rs` (compact JSON serializer) |
| `install` | `src/install.rs` (`--install` MCP client config generation) |
| `codegen` | `mcp-tools-codegen/` (proc-macro) and `build.rs` (tool-router generation) |
| `docs` | `docs/` (incl. `docs/plans/`) |
| `claude` | `.claude/` rules and agents |
| `make` | `Makefile` |
| `ci` | `.github/` |
| `deps` | `Cargo.toml`, `Cargo.lock`, `mcp-tools-codegen/Cargo.toml` |
| `app` | Cross-cutting changes, `src/main.rs`, `src/lib.rs`, `Dockerfile`, or a commit that legitimately spans multiple scopes |

Create **multiple logical commits** per PR. A commit spanning multiple scopes uses `app`.

Example:

```
feat(tools): add azdo_link_work_items tool with relation-type validation
```

### PR workflow (GitHub) — ABSOLUTE RULE

This repository is hosted on **GitHub**. PR and repo operations use the **GitHub CLI (`gh`)** — do NOT use `az repos`.

- Create the feature branch from the latest `main` before implementation (see `agent.md` §4).
- Branch naming: `feat/<short-description>` (or `fix/<...>`, `refactor/<...>`, etc.).
- Push commits regularly (at minimum after each user story or major task).
- When all plan work is complete and all quality gates pass, create the PR via `gh pr create` and report the PR URL to the user.

---

## Hard Project Invariants — ABSOLUTE RULES

These come from `docs/PROJECT.md` and `docs/ARCHITECTURE.md`. They MUST NOT be relaxed without explicit user direction.

- **Anti-prompt-injection — ABSOLUTE**: EVERY MCP tool MUST build its success `CallToolResult` via `tool_text_success()` (from `src/mcp/tools/support/`), which prepends the `UNTRUSTED_CONTENT_WARNING`. Using `CallToolResult::success(vec![Content::text(...)])` directly in tool files is FORBIDDEN. The warning MUST NEVER be removed, weakened, or shortened. (See `mcp.md`.)
- **No secrets, ever**: NEVER hardcode secrets, tokens, or passwords. Azure Bearer tokens are fetched per-request via `DefaultAzureCredential`, never persisted, and **NEVER logged at any level** (trace included). No secrets in code, docs, config, or logs.
- **Auth via `DefaultAzureCredential`**: credentials are resolved automatically (environment variables, managed identity, Azure CLI, etc.) with scope `499b84ac-1321-427f-aa17-267ca6975798`. Do NOT introduce alternative hardcoded credential paths.
- **API surface**: Azure DevOps REST API **v7.1** (Comments API `7.2-preview.4`); base URL `https://dev.azure.com/{organization}/{project}/_apis/`, VSSPS `https://app.vssps.visualstudio.com/_apis/`. Work-item fetching is batched (200 per batch, 1000 max).
- **Tool naming**: all MCP tools are prefixed **`azdo_`**. Keep names consistent with `docs/ARCHITECTURE.md` (MCP Tools table).
- **Transports**: stdio (default) and streamable HTTP (`--server`). HTTP binds `0.0.0.0:<port>` — deployment MUST be behind appropriate network controls. Do NOT change binding defaults without user direction.
- **`AzureDevOpsApi` trait boundary**: all MCP tool functions accept `&(dyn AzureDevOpsApi + Send + Sync)`, NOT `&AzureDevOpsClient`. API calls go through trait methods so integration tests can use `MockAzureDevOpsApi`. (See `mcp.md`.)
- **Idempotency**: MCP tool calls MUST be safe to retry.
- **Error handling**: NEVER `.unwrap()` in production code (only tests and `build.rs`); NEVER `panic!` in library code. (See `rust.md`.)

---

## Configuration

- All configuration is via **CLI flags (clap)** — see `docs/PROJECT.md` (Configuration). NEVER add environment variables for configuration except `RUST_LOG` (logging).

| Flag | Default | Purpose |
|---|---|---|
| `--server` | false | Run in HTTP server mode (default: stdio) |
| `--port` | 3000 | HTTP server port (only with `--server`) |
| `--install <client>` | — | Install MCP server configuration for a client (claude-code, claude-desktop, cursor, vscode, codex, gemini-cli) |

- Azure credentials are resolved via `DefaultAzureCredential` — never hardcoded.
- Logging is controlled via `RUST_LOG` (e.g. `RUST_LOG=debug`).
- NEVER hardcode secrets; NEVER log tokens.

The full dependency and configuration inventory lives in `docs/PROJECT.md` — that is canonical. Do NOT duplicate it here.

---

## Quality Gates

You MUST ALWAYS pass ALL quality gates in `rust.md` and satisfy the MCP/Azure tooling rules in `mcp.md` before considering any work done. Definition of Done, fix-broken-tests, fix-broken-linting, and no-suppression rules are defined in `rust.md`.

### Project Make targets

The canonical target list is in `docs/PROJECT.md` (Build, Lint, and Test Commands). The ones you will use most:

| Target | Purpose |
|---|---|
| `make build` | `cargo build` (debug) |
| `make release` | `cargo build --release` |
| `make run` | Run the binary |
| `make check` | `cargo check` |
| `make test` | `cargo test --features test-support` (unit + integration; `test-support` is REQUIRED — see below) |
| `make lint` | `cargo clippy --features test-support -- -D warnings` |
| `make fmt` | `cargo fmt` |
| `make clean` | `cargo clean` |
| `make all` | `fmt` → `lint` → `test` → `build` |

**`test-support` feature — ABSOLUTE**: `mockall` and the integration tests in `tests/*.rs` require the `test-support` feature, because `cfg(test)` is NOT active when the library is built as a dependency for integration tests, so `MockAzureDevOpsApi` would not be generated. ALWAYS use `make test` / `cargo test --features test-support` for the full suite — plain `cargo test` silently skips integration tests.

Per `agent.md`, during plan workflows linting and tests run ONLY at the end of the entire plan.

---

## Project Structure

Cargo workspace (two crates: root `mcp-for-azure-devops-boards` + `mcp-tools-codegen` proc-macro). Top-level layout:

```
Cargo.toml / Cargo.lock       # workspace root + main crate; pinned versions
build.rs                      # scans #[mcp_tool] → generates the tool router
Makefile  Dockerfile
src/
  main.rs  lib.rs  compact_llm.rs  install.rs
  azure/    # AzureDevOpsClient, AzureDevOpsApi trait, models, per-area API modules
  mcp/      # AzureMcpServer + ServerHandler; tools/ (per-tool impls + support/)
  server/   # HTTP transport (hyper + rmcp StreamableHttpService)
mcp-tools-codegen/            # #[mcp_tool] proc-macro
tests/                        # integration + install E2E (testcontainers) tests
docs/                         # PROJECT.md, ARCHITECTURE.md, plans/
.github/workflows/            # CI (PR build+test) + CD (tag build+release)
```

The detailed package inventory is in `docs/ARCHITECTURE.md` (Project Structure) — canonical. Do NOT create new top-level modules without updating that section.
