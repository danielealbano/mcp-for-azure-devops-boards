# Rust Rules — ABSOLUTE RULES

These rules apply to ALL Rust code in this repository. They are **VERY STRICT and ABSOLUTELY NON-NEGOTIABLE**!
MCP + Azure DevOps tooling specifics (tool conventions, anti-prompt-injection, the `AzureDevOpsApi` trait, code generation, transport, API integration) live in `mcp.md`. Project structure, dependencies, invariants, and Make targets live in `project.md`. Behavioral/process/git/plan rules live in `agent.md`.

## 1) Architecture & Idioms — ABSOLUTE RULES

### Rust idioms first
- You MUST prefer simplicity over cleverness. Clear is better than clever.
- You MUST follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and The Rust Book patterns.
- You MUST keep modules small and cohesive with narrow responsibilities. Prefer flat module trees. Avoid "util"/"common" mega-modules.
- You MUST use the type system to enforce invariants at compile time.
- You MUST prefer composition via traits over deep type hierarchies.
- You MUST export only what consumers need via `pub`; keep the public API surface minimal.
- You MUST prefer `Option<T>` / `Result<T, E>` to sentinel values; avoid `Option<T>` parameters where separate functions would be clearer.

### Interface-first (traits) and testability
- You MUST define traits for components that touch external systems (HTTP clients, the Azure DevOps API) to enable mocking in tests.
- You MUST use `#[async_trait]` for async trait methods.
- You MUST use `mockall` (optional dependency, enabled via the `test-support` feature) for generating mock implementations; for simple cases prefer hand-written mock structs implementing the trait.
- Keep traits small (1–3 methods). Prefer composing small traits over large ones.
- Traits/types use `PascalCase` with NO `I`/`T`-style Hungarian prefixes (`AzureDevOpsApi`, not `IAzureDevOpsApi`).

### Optionality and error surfaces
- You MUST prefer non-optional types; use `Option<T>` only when absence is a valid state.
- You MUST AVOID `.unwrap()` / `.expect()` in production code — use `?`, `match`, `if let`, `ok_or`, `map_err`, or the appropriate combinator. (`.unwrap()` is acceptable ONLY in tests and `build.rs`.)
- You MUST validate preconditions explicitly and return structured errors rather than panicking.

### Dependency injection
- You MUST pass dependencies explicitly via constructor parameters (`new()`).
- You MUST use `Arc<T>` for shared ownership across async tasks (e.g., `Arc<dyn AzureDevOpsApi + Send + Sync>` in `AzureMcpServer`).
- You MUST NEVER rely on mutable global state or `lazy_static!` for wiring dependencies. Compile-time constants via `once_cell::sync::Lazy` / `std::sync::LazyLock` (e.g., compiled-once regex patterns) are acceptable.

### Concurrency and race conditions
The MCP server handles concurrent requests (HTTP mode spawns a task per connection; each tool invocation runs concurrently). You MUST ALWAYS assume the system runs concurrently.

You MUST:
- use `Arc<T>` for shared immutable state across async tasks,
- use `Arc<Mutex<T>>` or `tokio::sync::Mutex<T>` for shared mutable state,
- NEVER use blocking operations in async context — use `tokio::task::spawn_blocking` if unavoidable,
- NEVER spawn fire-and-forget tasks without a clear shutdown path,
- design for idempotency where appropriate and handle retries without duplicate side effects,
- leverage Rust's ownership system — the compiler prevents most data races at compile time.

## 2) Coding Standards — ABSOLUTE RULES

### Naming
- Functions/variables/modules: `snake_case`. Types/traits/enums/variants: `PascalCase`. Constants/statics: `SCREAMING_SNAKE_CASE`.

### Async
- All I/O functions are `async` (tokio, full features). Use `tokio::spawn` for concurrent tasks.
- NEVER block the async runtime; offload unavoidable blocking work with `tokio::task::spawn_blocking`.

### Validation
- You MUST validate inputs at the boundary. For MCP tool parameters, validate type, range, and required fields via serde deserialization and custom deserializers (e.g., `deserialize_non_empty_string`), and return structured MCP errors for invalid params (see `mcp.md`).

### Error handling
- You MUST use `thiserror` for domain error enums (e.g., `AzureError` with `AuthError`, `HttpError`, `SerdeJson`, `ApiError` variants).
- You MUST use `anyhow::Result` for application-level errors (e.g., in `main()`).
- You MUST use the `?` operator for propagation; add context via `.map_err()` or error-enum variants.
- You MUST convert domain errors to MCP errors via `.map_err(...)` at the MCP layer. Do NOT leak internal paths, tokens, or raw upstream bodies to MCP clients — sanitize error messages.
- You MUST NEVER silently discard errors (e.g., `let _ = fallible_call();`) without a documented justification.
- You MUST NEVER `panic!` in library code. Panics are acceptable only for truly unrecoverable programmer errors.
- Return errors, don't log-and-continue, unless the error is truly informational.

### Logging
- Use the `log` crate (facade) with `env_logger` as the backend; configured in `main()` via `env_logger::init()`; controlled via `RUST_LOG`.
- Log levels: `trace` (fine-grained), `debug` (internal flow), `info` (business events), `warn` (recoverable), `error` (unrecoverable).
- You MUST NEVER log secrets, tokens, API keys, or PII — at ANY level.
- Errors must be actionable: include what failed, which identifiers, and likely next steps.

### Configuration & dependencies
- NEVER hardcode secrets or environment-specific values. All configuration is via CLI flags (clap); the only config env var is `RUST_LOG` (see `project.md`).
- Keep `Cargo.toml` + `Cargo.lock` as the single source of dependency versions; commit both. Use latest stable versions unless an in-use package requires an older release; verify a version is current before adding (see `agent.md` §1bis for verifying external claims). No `[patch]` sections, no local forks.
- Prefer the Rust standard library over third-party crates when feasible; wrap third-party clients behind project-owned traits for testability.

## 3) Testing Rules — ABSOLUTE RULES

All references to "tests" mean automated tests (unit, integration, e2e) that run in development and CI.

### General principles
- Tests are MANDATORY for all changes. There are ZERO exceptions.
- Tests MUST be small, focused, and non-redundant while covering the happy path, edge cases (empty inputs, boundary values, `None` variants), and failure modes (error returns, invalid data, network failures).
- Tests MUST ALWAYS pass, MUST NOT depend on execution order, and MUST clean up after themselves (temp files, connections).

### Unit tests
- Live inline in `#[cfg(test)] mod tests` blocks within the source file they test.
- MUST be fast: no I/O, no network, no external services.
- Use `#[test]` for sync tests, `#[tokio::test]` for async tests.
- Name tests descriptively: `test_type_name_method_name_scenario` (snake_case).
- Use `assert!`, `assert_eq!`, `assert_ne!` with descriptive messages.
- Use `mockall` (via `test-support`) or hand-written trait mocks for external dependencies. Use `tempfile` for filesystem-dependent tests.
- **Parameterized tests are the default** for functions with multiple input/output cases: define a vector of named cases and iterate, asserting with a message that identifies the case.

```rust
#[test]
fn test_parse_something_variants() {
    let cases = vec![
        ("valid input", "ok", false),
        ("missing field", "", true),
    ];
    for (name, input, want_err) in cases {
        let result = parse_something(input);
        assert_eq!(result.is_err(), want_err, "case '{}': input={}", name, input);
    }
}
```

### Integration tests
- Live in `tests/*.rs` at the crate root.
- Use `MockAzureDevOpsApi` (generated by `mockall` under the `test-support` feature) or hand-written mock HTTP servers — NEVER real external services.
- **`test-support` feature — ABSOLUTE**: `cfg(test)` is NOT active when the library is built as a dependency for `tests/*.rs`, so `MockAzureDevOpsApi` is generated ONLY when `test-support` is enabled. ALWAYS run `make test` / `cargo test --features test-support`; plans adding integration tests MUST enable this feature. Plain `cargo test` silently skips integration tests.
- Shared test infrastructure (mock servers, common helpers like `assert_tool_output_has_warning` in `tests/common/`) is foundational — reuse it; do NOT hand-roll per-test wiring.

### Environment variables for tests
- Tests that need env-based config load a gitignored `.env` (via `dotenv`); Make targets source it automatically. Never commit real credentials.

### Manual testing
- Manual tests are NOT a substitute for automated tests. If manual steps are necessary, label them clearly as **Manual Test** / **Manual QA Steps**, separate from automated test descriptions.

## 4) Quality Gates — ABSOLUTE RULES

### Definition of Done
A change is DONE **ONLY AND ONLY** if ALL are true:

- All relevant automated tests written AND passing (unit + integration as appropriate) via `make test`.
- No linting warnings/errors: `make lint` (`cargo clippy --features test-support -- -D warnings`).
- The project builds without errors and without warnings: `make build` (`cargo build`).
- No TODOs, no commented-out dead code, no placeholders, no stubs, no "temporary hacks".
- No `.unwrap()` in production code (only tests and `build.rs`); no `panic!` in library code.
- Changes are small, readable, and aligned with existing Rust patterns.
- All MCP tool conventions and the anti-prompt-injection requirement in `mcp.md` are satisfied.

### Fix broken tests — ABSOLUTE RULE
- You MUST fix ANY broken test, even if unrelated to your change. Finish your current change first, then fix it immediately. NEVER leave the suite broken. ZERO exceptions.

### Fix broken linting — ABSOLUTE RULE
- You MUST fix ANY linting/formatting error, even if unrelated. Finish your change first, then fix it. NEVER leave violations. ZERO exceptions.

### No linting suppression — ABSOLUTE RULE
- You MUST NEVER suppress, silence, or skip linting rules (`#[allow(...)]` attributes, disabling rules in `clippy.toml`) to make errors disappear. FIX the root cause by adjusting the implementation.
- The ONLY exception: a genuine, unavoidable conflict with a documented design decision. You MUST explain it and get EXPLICIT user approval before adding any suppression. NON-NEGOTIABLE.

### Standard commands
- Build: `make build` (`cargo build`) · Lint: `make lint` (`cargo clippy --features test-support -- -D warnings`) · Format: `make fmt` (`cargo fmt`) · Tests (unit + integration): `make test` (`cargo test --features test-support`).
- Project-specific Make targets are in `project.md` and `docs/PROJECT.md`.
- Rust tools (`cargo`, `rustc`, `rustfmt`, `cargo-clippy`) are managed by `rustup`, typically in `~/.cargo/bin`. If a tool is missing, check `~/.cargo/bin` first; if still missing, ASK the user to install it — do NOT install it yourself.
