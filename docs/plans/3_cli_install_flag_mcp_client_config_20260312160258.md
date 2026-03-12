<!-- SACRED DOCUMENT — DO NOT MODIFY except for checkmarks ([ ] → [x]) and review findings. -->
<!-- You MUST NEVER alter, revert, or delete files outside the scope of this plan. -->
<!-- Plans in docs/plans/ are PERMANENT artifacts. There are ZERO exceptions. -->

# Plan: CLI `--install` Flag and MCP Client Configuration

---

## User Story 1: CLI `--install` flag with multi-client configuration support

Allow users to register the MCP server with their preferred AI client via a single `--install <target>` command that auto-detects the binary path, resolves the correct config file, and writes the entry in the expected format.

### Acceptance Criteria

- [x] `--install <target>` flag added (targets: `claude-code`, `claude-desktop`, `cursor`, `vscode`, `codex`, `gemini-cli`)
- [x] Detects binary path via `std::env::current_exe()`
- [x] Resolves config file path per target and platform
- [x] Creates/updates config file with correct format per target
- [x] Mutually exclusive with `--server`
- [x] Preserves existing config file content (other keys, other server entries)
- [x] Updates existing server entry if already present (path changed)
- [x] Creates parent directories if they don't exist
- [x] Prints success message with config file path and exits
- [x] Unit tests for all targets and edge cases

---

### Task 1.1: Add runtime dependencies

**Action**: Modify `Cargo.toml`

Add to `[dependencies]`:

```toml
dirs = "6"
toml = "0.8"
```

Add to `[dev-dependencies]`:

```toml
tempfile = "3"
```

**Definition of Done**:

- [x] `dirs`, `toml` added to `[dependencies]`
- [x] `tempfile` added to `[dev-dependencies]`

---

### Task 1.2: Create install module

**Action**: Create `src/install.rs`

#### `InstallError` enum

Uses `thiserror`. Variants:

| Variant | Fields | Message |
|---------|--------|---------|
| `HomeDirectoryNotFound` | — | `Could not determine home directory` |
| `ConfigDirectoryNotFound` | — | `Could not determine config directory` |
| `CurrentDirectoryNotFound` | `source: std::io::Error` | `Could not determine current directory: {source}` |
| `BinaryPathDetection` | `source: std::io::Error` | `Could not detect binary path: {source}` |
| `CreateDirectory` | `path: PathBuf, source: std::io::Error` | `Failed to create directory {path}: {source}` |
| `ReadConfig` | `path: PathBuf, source: std::io::Error` | `Failed to read config file {path}: {source}` |
| `WriteConfig` | `path: PathBuf, source: std::io::Error` | `Failed to write config file {path}: {source}` |
| `ParseJson` | `path: PathBuf, source: serde_json::Error` | `Failed to parse JSON config {path}: {source}` |
| `ParseToml` | `path: PathBuf, source: toml::de::Error` | `Failed to parse TOML config {path}: {source}` |
| `SerializeToml` | `source: toml::ser::Error` | `Failed to serialize TOML config: {source}` |
| `InvalidConfigFormat` | `path: PathBuf, detail: String` | `Invalid config format in {path}: {detail}` |

#### `InstallTarget` enum

Derives `Clone`, `Debug`, `clap::ValueEnum`.

| Variant | CLI value | Display |
|---------|-----------|---------|
| `ClaudeCode` | `claude-code` | `Claude Code` |
| `ClaudeDesktop` | `claude-desktop` | `Claude Desktop` |
| `Cursor` | `cursor` | `Cursor` |
| `Vscode` | `vscode` | `VS Code` |
| `Codex` | `codex` | `Codex CLI` |
| `GeminiCli` | `gemini-cli` | `gemini-cli` |

Implement `std::fmt::Display` for user-facing success messages.

#### Constants

```rust
const SERVER_NAME: &str = "mcp-for-azure-devops-boards";
```

#### `resolve_config_path(target: &InstallTarget) -> Result<PathBuf, InstallError>`

| Target | Resolution |
|--------|-----------|
| `ClaudeCode` | `dirs::home_dir()? / ".claude.json"` |
| `Cursor` | `dirs::home_dir()? / ".cursor" / "mcp.json"` |
| `GeminiCli` | `dirs::home_dir()? / ".gemini" / "settings.json"` |
| `Codex` | `dirs::home_dir()? / ".codex" / "config.toml"` |
| `Vscode` | `std::env::current_dir()? / ".vscode" / "mcp.json"` |
| `ClaudeDesktop` | `dirs::config_dir()? / "Claude" / "claude_desktop_config.json"` |

Note: `dirs::config_dir()` resolves platform-correctly:
- macOS: `~/Library/Application Support`
- Linux: `~/.config`
- Windows: `%APPDATA%`

#### `install(target: &InstallTarget, config_path: &Path, binary_path: &Path) -> Result<String, InstallError>`

1. Create parent directories of `config_path` if they don't exist (`std::fs::create_dir_all`)
2. Dispatch to format-specific function based on target:
   - `Codex` → `install_toml(config_path, binary_path)`
   - `Vscode` → `install_json(config_path, binary_path, "servers", true)`
   - All others → `install_json(config_path, binary_path, "mcpServers", false)`
3. Return success message: `"Installed {target} configuration at {config_path}"`

#### `install_json(config_path: &Path, binary_path: &Path, servers_key: &str, include_type_stdio: bool) -> Result<(), InstallError>`

1. Read existing file content. If file doesn't exist OR is empty (0 bytes), use `"{}"` as default. Distinguish "not found" from other IO errors.
2. Parse as `serde_json::Value`. If the root value is not a JSON Object, return `InstallError::InvalidConfigFormat` with detail `"expected JSON object at root"`.
3. Ensure `servers_key` exists as an Object (create if missing)
4. Build server entry as `serde_json::Value`:
   - If `include_type_stdio`: `{ "type": "stdio", "command": "<binary_path>" }`
   - Else: `{ "command": "<binary_path>" }`
   - Convert `binary_path` to string using `to_string_lossy()` for JSON string values
5. Insert/overwrite entry at key `SERVER_NAME`
6. Serialize with `serde_json::to_string_pretty`
7. Write to `config_path` (append trailing newline)

#### `install_toml(config_path: &Path, binary_path: &Path) -> Result<(), InstallError>`

1. Read existing file content. If file doesn't exist OR is empty (0 bytes), use `""` as default.
2. Parse as `toml::Value::Table`. Convert `binary_path` to string using `to_string_lossy()` for TOML string values.
3. Ensure `mcp_servers` exists as a Table (create if missing)
4. Build server entry as `toml::Value::Table` with key `command` = `binary_path` string
5. Insert/overwrite entry at key `SERVER_NAME`
6. Serialize with `toml::to_string_pretty`
7. Write to `config_path`

#### Expected output formats

**Claude Code** (`~/.claude.json`), **Claude Desktop** (`<config_dir>/Claude/claude_desktop_config.json`), **Cursor** (`~/.cursor/mcp.json`), **gemini-cli** (`~/.gemini/settings.json`):

```json
{
  "mcpServers": {
    "mcp-for-azure-devops-boards": {
      "command": "/path/to/mcp-for-azure-devops-boards"
    }
  }
}
```

**VS Code** (`.vscode/mcp.json`):

```json
{
  "servers": {
    "mcp-for-azure-devops-boards": {
      "type": "stdio",
      "command": "/path/to/mcp-for-azure-devops-boards"
    }
  }
}
```

**Codex CLI** (`~/.codex/config.toml`):

```toml
[mcp_servers.mcp-for-azure-devops-boards]
command = "/path/to/mcp-for-azure-devops-boards"
```

**Definition of Done**:

- [x] Module compiles without warnings
- [x] All 6 targets supported with correct format
- [x] JSON targets use correct key (`mcpServers` vs `servers`)
- [x] VS Code entries include `"type": "stdio"`
- [x] Codex uses TOML format
- [x] Parent directories created when needed
- [x] Existing config content preserved
- [x] Existing server entry updated on re-install
- [x] Error variants cover all failure modes (including `InvalidConfigFormat` for non-Object JSON root)

---

### Task 1.3: Expose install module

**Action**: Modify `src/lib.rs` — add `pub mod install;`

**Definition of Done**:

- [x] Module accessible from `main.rs` and integration tests

---

### Task 1.4: Integrate install flag into CLI

**Action**: Modify `src/main.rs`

Add import at the top of `src/main.rs`:

```rust
use mcp_for_azure_devops_boards::install::{InstallTarget, install, resolve_config_path, InstallError};
```

Add to `Args` struct:

```rust
/// Install MCP server configuration for the specified client
#[arg(long, value_enum, conflicts_with = "server")]
install: Option<InstallTarget>,
```

Add to `main()` body, **immediately after** `let args = Args::parse();` and **before** `let client = AzureDevOpsClient::new();` — the install path must NOT create the Azure client or MCP server since it is a pure file-write operation:

```rust
if let Some(target) = &args.install {
    let binary_path = std::env::current_exe()
        .map_err(InstallError::BinaryPathDetection)?;
    let config_path = resolve_config_path(target)?;
    let message = install(target, &config_path, &binary_path)?;
    println!("{message}");
    return Ok(());
}
```

**Definition of Done**:

- [x] `--install <target>` flag accepted by CLI
- [x] Conflicts with `--server` (clap error if both provided)
- [x] Process exits after successful install with message
- [x] Error propagated correctly on failure

---

### Task 1.5: Unit tests for install module

**File**: `src/install.rs` (`#[cfg(test)] mod tests`)

**Setup**: `tempfile::TempDir` for isolated file system. Binary path simulated as `/test/path/mcp-for-azure-devops-boards`.

| Test | Verifies |
|------|----------|
| `test_install_claude_code_creates_config` | Creates file with `{"mcpServers":{"mcp-for-azure-devops-boards":{"command":"..."}}}` from scratch |
| `test_install_claude_desktop_creates_config` | Creates file with `{"mcpServers":{"mcp-for-azure-devops-boards":{"command":"..."}}}` from scratch |
| `test_install_cursor_creates_config` | Creates file with `{"mcpServers":{"mcp-for-azure-devops-boards":{"command":"..."}}}` from scratch |
| `test_install_gemini_cli_creates_config` | Creates file with `{"mcpServers":{"mcp-for-azure-devops-boards":{"command":"..."}}}` from scratch |
| `test_install_vscode_creates_config` | Creates file with `{"servers":{"mcp-for-azure-devops-boards":{"type":"stdio","command":"..."}}}` from scratch |
| `test_install_codex_creates_config` | Creates TOML file with `[mcp_servers.mcp-for-azure-devops-boards]` and `command = "..."` from scratch |
| `test_install_json_preserves_existing_keys` | Pre-existing top-level keys (e.g. `"theme": "dark"`) remain after install |
| `test_install_json_preserves_other_servers` | Pre-existing server entries in the `mcpServers` object remain after install |
| `test_install_json_updates_existing_entry` | Re-running install with a different binary path updates the `command` value |
| `test_install_toml_preserves_existing_keys` | Pre-existing TOML keys remain after install |
| `test_install_toml_preserves_other_servers` | Pre-existing server entries in `mcp_servers` table remain after install |
| `test_install_toml_updates_existing_entry` | Re-running install with a different binary path updates the `command` value |
| `test_install_creates_parent_directories` | Config file written successfully when parent directories don't exist |
| `test_install_returns_success_message` | Return value contains target name and config file path |
| `test_install_json_invalid_json_returns_error` | Existing file with malformed JSON (e.g. `{broken`) returns `ParseJson` error |
| `test_install_toml_invalid_toml_returns_error` | Existing file with malformed TOML returns `ParseToml` error |
| `test_install_json_empty_file_treated_as_new` | Existing 0-byte file is treated as `{}` — install succeeds and produces correct config |
| `test_install_json_root_not_object_returns_error` | Existing file with JSON array (`[]`) or primitive returns `InvalidConfigFormat` error |
| `test_install_toml_empty_file_treated_as_new` | Existing 0-byte TOML file is treated as empty table — install succeeds and produces correct config |
| `test_resolve_config_path_claude_code` | Path ends with `.claude.json` (assert suffix, not full path, to avoid host dependency) |
| `test_resolve_config_path_cursor` | Path ends with `.cursor/mcp.json` |
| `test_resolve_config_path_gemini_cli` | Path ends with `.gemini/settings.json` |
| `test_resolve_config_path_codex` | Path ends with `.codex/config.toml` |
| `test_resolve_config_path_vscode` | Path ends with `.vscode/mcp.json` |
| `test_resolve_config_path_claude_desktop` | Path ends with `Claude/claude_desktop_config.json` |

**File**: `src/main.rs` (`#[cfg(test)] mod tests`) — extend existing tests

| Test | Verifies |
|------|----------|
| `test_install_flag_parsing` | `--install claude-code` parses correctly |
| `test_install_conflicts_with_server` | `--install claude-code --server` is rejected by clap |
| `test_install_all_targets_parse` | All 6 target values parse correctly |

**Definition of Done**:

- [x] All 6 targets have config creation tests
- [x] Edge cases covered (existing config, re-install, directory creation, malformed input, empty files)
- [x] Path resolution tested for all targets (suffix assertions)

---

## User Story 2: E2E tests with testcontainers

Verify config format compatibility with real CLI tools using Docker containers. Tests generate config with our install logic, inject it into a container running the real CLI tool, and verify the tool recognizes the server.

### Acceptance Criteria

- [ ] `testcontainers` dev-dependency added
- [ ] Dockerfiles for test images (Claude Code, Cursor CLI, gemini-cli)
- [ ] E2E tests for `claude-code`, `cursor`, `gemini-cli`
- [ ] Tests verify real CLI tools recognize our generated config
- [ ] Tests marked `#[ignore]` (opt-in via `cargo test --ignored`)
- [ ] Makefile target for building test images and running E2E tests

---

### Task 2.1: Add testcontainers dev-dependency

**Action**: Modify `Cargo.toml`

Add to `[dev-dependencies]`:

```toml
testcontainers = "0.27"
```

**Definition of Done**:

- [ ] `testcontainers` added to `[dev-dependencies]`

---

### Task 2.2: Create Dockerfiles for test images

**Action**: Create `tests/docker/claude-code/Dockerfile`

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://claude.ai/install.sh | bash
ENV PATH="/root/.local/bin:${PATH}"
```

**Action**: Create `tests/docker/cursor/Dockerfile`

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl https://cursor.com/install -fsSL | bash
ENV PATH="/root/.local/bin:${PATH}"
```

**Action**: Create `tests/docker/gemini-cli/Dockerfile`

```dockerfile
FROM node:20-slim
RUN npm install -g @google/gemini-cli
```

**Definition of Done**:

- [ ] All 3 Dockerfiles build successfully
- [ ] CLI tools accessible inside containers (verified by running version commands)

---

### Task 2.3: Create E2E test file

**Action**: Create `tests/test_install_e2e.rs`

**Setup**: Each test is `#[ignore]` + `#[tokio::test]`.

**Flow per test**:

1. Call `install()` from the library with a temp directory path as the config location
2. Read the generated config file content from the temp directory
3. Start a testcontainer from the pre-built Docker image
4. Write the config content into the container at the path the CLI tool expects (via exec using `echo` + base64 decode, or equivalent)
5. Execute the CLI tool's list command inside the container
6. Assert the output contains `mcp-for-azure-devops-boards`

| Test | CLI tool | Config path inside container | Verification command |
|------|----------|------------------------------|---------------------|
| `test_e2e_claude_code_recognizes_config` | `claude` | `/root/.claude.json` | `claude mcp list` |
| `test_e2e_cursor_recognizes_config` | `agent` | `/root/.cursor/mcp.json` | `agent mcp list` |
| `test_e2e_gemini_cli_recognizes_config` | `gemini` | `/root/.gemini/settings.json` | `gemini mcp list` |

**Definition of Done**:

- [ ] Tests are `#[ignore]` and do not run with `cargo test`
- [ ] 3 E2E tests created (one per CLI-verifiable target)

---

### Task 2.4: Add Makefile targets

**Action**: Modify `Makefile`

1. Update `.PHONY` declaration (line 1) to include new targets:

```makefile
.PHONY: all build release run check test lint fmt clean help test-e2e-build test-e2e
```

2. Add two targets after the `clean` target:

```makefile
test-e2e-build:
	docker build -t mcp-test-claude-code tests/docker/claude-code/
	docker build -t mcp-test-cursor tests/docker/cursor/
	docker build -t mcp-test-gemini-cli tests/docker/gemini-cli/

test-e2e: test-e2e-build
	cargo test --features test-support --ignored
```

3. Update the `help` target to include the new entries:

```makefile
	@echo "  test-e2e-build - Build Docker images for E2E tests"
	@echo "  test-e2e       - Run E2E testcontainers tests (requires Docker)"
```

**Definition of Done**:

- [ ] `.PHONY` updated with `test-e2e-build` and `test-e2e`
- [ ] `make test-e2e-build` builds all 3 Docker images
- [ ] `make test-e2e` builds images and runs E2E tests
- [ ] `make help` shows new targets
- [ ] Existing Makefile targets unaffected

---

## User Story 3: README and documentation update

Provide clear configuration examples for all supported MCP clients so users can set up the tool without guesswork.

### Acceptance Criteria

- [ ] README MCP Configuration section restructured with all 6 targets
- [ ] macOS (Homebrew) and Windows (Scoop) binary paths shown
- [ ] `--install` flag documented as quick setup alternative
- [ ] `docs/PROJECT.md` updated with new dependencies and `--install` flag
- [ ] `docs/ARCHITECTURE.md` updated with `src/install.rs`

---

### Task 3.1: Restructure README MCP Configuration section

**Action**: Modify `README.md`

Replace the current "MCP Configuration" section (lines 101–117, which only covers Claude Desktop) with a restructured section containing:

1. **Quick setup with `--install`**: Show `mcp-for-azure-devops-boards --install <target>` as the fastest way. List all 6 valid target values.

2. **Manual configuration** subsections for each target, in this order:
   - **Claude Code**: Config at `~/.claude.json`. JSON with `mcpServers` key. Show macOS/Linux path (`/opt/homebrew/bin/mcp-for-azure-devops-boards`) and Windows path (`%USERPROFILE%\scoop\apps\mcp-for-azure-devops-boards\current\mcp-for-azure-devops-boards.exe`).
   - **Claude Desktop**: Config at platform-specific paths (macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`, Linux: `~/.config/Claude/claude_desktop_config.json`, Windows: `%APPDATA%\Claude\claude_desktop_config.json`). JSON with `mcpServers` key.
   - **Cursor**: Config at `~/.cursor/mcp.json`. JSON with `mcpServers` key.
   - **VS Code**: Config at `.vscode/mcp.json` (workspace level). JSON with `servers` key and `"type": "stdio"`.
   - **gemini-cli**: Config at `~/.gemini/settings.json`. JSON with `mcpServers` key.
   - **Codex CLI**: Config at `~/.codex/config.toml`. TOML with `[mcp_servers.mcp-for-azure-devops-boards]` section.

Each manual config entry: one JSON/TOML code block with the macOS/Linux path, and a note with the Windows Scoop path to substitute. Keep it concise — no duplication of full blocks for each OS.

**Definition of Done**:

- [ ] All 6 targets documented
- [ ] Both macOS (Homebrew) and Windows (Scoop) paths referenced
- [ ] `--install` flag documented
- [ ] Section is concise

---

### Task 3.2: Update docs/PROJECT.md

**Action**: Modify `docs/PROJECT.md`

- Add `dirs` and `toml` to the Runtime dependencies table
- Add `testcontainers` and `tempfile` to the Dev dependencies table
- Add `--install` row to the CLI flags table:

  | Flag | Default | Description |
  |------|---------|-------------|
  | `--install` | — | Install MCP server configuration for the specified client (claude-code, claude-desktop, cursor, vscode, codex, gemini-cli) |

**Definition of Done**:

- [ ] Runtime dependencies table has `dirs` and `toml`
- [ ] Dev dependencies table has `testcontainers` and `tempfile`
- [ ] CLI flags table has `--install`

---

### Task 3.3: Update docs/ARCHITECTURE.md

**Action**: Modify `docs/ARCHITECTURE.md`

- Add `src/install.rs` to the project structure tree (under `src/`, after `compact_llm.rs`):
  ```
  │   ├── install.rs                # CLI --install: config generation for MCP clients
  ```
- Add `test_install_e2e.rs` and `tests/docker/` to the project structure tree:
  ```
  │   ├── test_install_e2e.rs       # E2E testcontainers tests for install config format
  │   ├── docker/                   # Dockerfiles for E2E testcontainers tests
  │   │   ├── claude-code/Dockerfile
  │   │   ├── cursor/Dockerfile
  │   │   └── gemini-cli/Dockerfile
  ```
- Update the `main.rs` line in the project structure to mention install flag:
  ```
  │   ├── main.rs                   # CLI entry (clap), transport selection, --install
  ```

**Definition of Done**:

- [ ] `install.rs` appears in project structure
- [ ] `tests/docker/` appears in project structure
- [ ] `main.rs` description updated

---

## User Story 4: Final verification

Verify the entire implementation end-to-end to ensure all quality gates pass and all behavior matches the plan.

### Acceptance Criteria

- [ ] All quality gates pass (`make fmt`, `make lint`, `make test`, `make build`)
- [ ] All install targets produce correct output
- [ ] E2E Docker images build and tests pass
- [ ] Documentation updated and accurate
- [ ] No code quality issues

### Task 4.1: End-to-end verification of entire implementation

#### Build and quality gates

- [ ] `make fmt` passes (no formatting issues)
- [ ] `make lint` passes (no clippy warnings)
- [ ] `make test` passes (all unit + integration tests)
- [ ] `make build` passes (no build warnings)

#### Install flag behavior

- [ ] `--install claude-code` produces correct JSON at `~/.claude.json` with `mcpServers` key
- [ ] `--install claude-desktop` produces correct JSON at `<config_dir>/Claude/claude_desktop_config.json` with `mcpServers` key
- [ ] `--install cursor` produces correct JSON at `~/.cursor/mcp.json` with `mcpServers` key
- [ ] `--install vscode` produces correct JSON at `.vscode/mcp.json` with `servers` key and `"type": "stdio"`
- [ ] `--install gemini-cli` produces correct JSON at `~/.gemini/settings.json` with `mcpServers` key
- [ ] `--install codex` produces correct TOML at `~/.codex/config.toml` with `[mcp_servers.mcp-for-azure-devops-boards]`
- [ ] `--install` and `--server` together are rejected by clap
- [ ] `--install` with invalid target is rejected by clap
- [ ] Existing config file content is preserved after install
- [ ] Re-running install updates the binary path
- [ ] Parent directories are created if missing

#### E2E tests

- [ ] `make test-e2e-build` builds all 3 Docker images without errors
- [ ] `make test-e2e` runs and all E2E tests pass

#### Documentation

- [ ] README MCP Configuration section covers all 6 targets with correct formats
- [ ] README mentions `--install` flag
- [ ] `docs/PROJECT.md` has updated dependencies and CLI flag tables
- [ ] `docs/ARCHITECTURE.md` has `install.rs` and `tests/docker/` in project structure

#### Code quality

- [ ] No TODOs in code
- [ ] No dead code or commented-out code
- [ ] No temporary hacks
- [ ] Error handling covers all failure modes
- [ ] Success messages are clear and include the config file path
