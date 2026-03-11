<!-- SACRED DOCUMENT — DO NOT MODIFY except for checkmarks ([ ] → [x]) and review findings. -->
<!-- You MUST NEVER alter, revert, or delete files outside the scope of this plan. -->
<!-- Plans in docs/plans/ are PERMANENT artifacts. There are ZERO exceptions. -->

# Plan 2 — Codebase Quality, Security & Performance Improvements

## Excluded from this plan (agreed with user)

- S4 (HTTP server security — not necessary, behind proxy)
- S7 (Raw WIQL intentional — Azure DevOps validates)
- Q8 (Build script parser — deferred to separate plan)
- P6 (Token caching — Azure SDK already caches internally)
- P5 (opt-level change — binary size becomes too large with opt-level 3)
- Q4/build.rs tests (deferred with Q8)

---

## US1: Security — URL Encoding, WIQL Escaping, JSON Pointer Escaping

Prevents URL injection, WIQL injection, and invalid JSON Patch paths.

### Acceptance Criteria

- [ ] All user-controlled strings interpolated into URLs are percent-encoded
- [ ] WIQL project and date fields use single-quote escaping
- [ ] JSON Patch field names are RFC 6901 escaped

---

### T1.1: URL-encode path segments in `client.rs`

**File**: `src/azure/client.rs` — modify

Encode `organization`, `project`, and `team` wherever they are interpolated into URL base paths. The `path` parameter is NOT encoded (it contains slashes, query strings, etc. constructed by API modules).

```rust
// In request_with_content_type (and get_with_headers, post_binary, get_binary):
let url = format!(
    "https://dev.azure.com/{}/{}/_apis/{}",
    urlencoding::encode(organization),
    urlencoding::encode(project),
    path
);

// In org_request:
let url = format!(
    "https://dev.azure.com/{}/_apis/{}",
    urlencoding::encode(organization),
    path
);

// In team_request:
let url = format!(
    "https://dev.azure.com/{}/{}/{}/_apis/{}",
    urlencoding::encode(organization),
    urlencoding::encode(project),
    urlencoding::encode(team),
    path
);
```

**DoD**:

- [ ] `request_with_content_type`: `organization` and `project` encoded
- [ ] `org_request`: `organization` encoded
- [ ] `team_request`: `organization`, `project`, `team` encoded
- [ ] `get_with_headers`: `organization` and `project` encoded
- [ ] `post_binary`: `organization` and `project` encoded
- [ ] `get_binary`: `organization` and `project` encoded
- [ ] `vssps_request` unchanged (no user-controlled path segments in base URL)

---

### T1.2: URL-encode parameters in API module paths

Encode user-controlled values interpolated into the `path` string passed to client methods. `urlencoding` is already a dependency.

**File**: `src/azure/boards.rs` — modify

```rust
// list_teams: encode project in path
let path = format!("projects/{}/teams?api-version=7.1", urlencoding::encode(project));

// get_team: encode project and team_id
let path = format!(
    "projects/{}/teams/{}?api-version=7.1",
    urlencoding::encode(project),
    urlencoding::encode(team_id)
);

// get_board: encode board_id
let path = format!("work/boards/{}?api-version=7.1", urlencoding::encode(board_id));

// list_board_columns: encode board_id
let path = format!("work/boards/{}/columns?api-version=7.1", urlencoding::encode(board_id));

// list_board_rows: encode board_id
let path = format!("work/boards/{}/rows?api-version=7.1", urlencoding::encode(board_id));
```

**File**: `src/azure/teams.rs` — modify

```rust
// list_team_members: encode project and team_id
let path = format!(
    "projects/{}/teams/{}/members?api-version=7.1",
    urlencoding::encode(project),
    urlencoding::encode(team_id)
);
```

**File**: `src/azure/organizations.rs` — modify

```rust
// list_organizations: encode member_id (query param value)
let path = format!("accounts?memberId={}&api-version=7.1", urlencoding::encode(member_id));
```

**File**: `src/azure/iterations.rs` — modify

```rust
// get_team_iterations: encode timeframe query param
let path = if let Some(tf) = timeframe {
    format!(
        "work/teamsettings/iterations?$timeframe={}&api-version=7.1",
        urlencoding::encode(tf)
    )
} else {
    "work/teamsettings/iterations?api-version=7.1".to_string()
};
```

**File**: `src/azure/work_items.rs` — modify

```rust
// get_comments: encode continuation_token
if let Some(token) = &continuation_token {
    path.push_str(&format!("&continuationToken={}", urlencoding::encode(token)));
}

// create_work_item: encode work_item_type
let path = format!("wit/workitems/${}?api-version=7.1", urlencoding::encode(work_item_type));
```

**DoD**:

- [ ] `boards.rs`: `project`, `team_id`, `board_id` encoded in all paths
- [ ] `teams.rs`: `project`, `team_id` encoded
- [ ] `organizations.rs`: `member_id` encoded
- [ ] `iterations.rs`: `timeframe` encoded
- [ ] `work_items.rs`: `continuation_token` and `work_item_type` encoded

---

### T1.3: Fix WIQL injection — escape project and date fields

**File**: `src/mcp/tools/work_items/query_work_items.rs` — modify

Apply `.replace("'", "''")` to `args.project` and all six date fields:

```rust
// Project field (in the empty-conditions fallback):
format!(
    "SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = '{}'",
    args.project.replace("'", "''")
)

// All date filters — same pattern for each:
if let Some(date) = &args.created_date_from {
    conditions.push(format!("[System.CreatedDate] >= '{}'", date.replace("'", "''")));
}
if let Some(date) = &args.created_date_to {
    conditions.push(format!("[System.CreatedDate] <= '{}'", date.replace("'", "''")));
}
if let Some(date) = &args.state_change_date_from {
    conditions.push(format!(
        "[Microsoft.VSTS.Common.StateChangeDate] >= '{}'",
        date.replace("'", "''")
    ));
}
if let Some(date) = &args.state_change_date_to {
    conditions.push(format!(
        "[Microsoft.VSTS.Common.StateChangeDate] <= '{}'",
        date.replace("'", "''")
    ));
}
if let Some(date) = &args.changed_date_from {
    conditions.push(format!("[System.ChangedDate] >= '{}'", date.replace("'", "''")));
}
if let Some(date) = &args.changed_date_to {
    conditions.push(format!("[System.ChangedDate] <= '{}'", date.replace("'", "''")));
}
```

**DoD**:

- [ ] `args.project` escaped in WIQL WHERE clause
- [ ] All 6 date fields (`created_date_from/to`, `state_change_date_from/to`, `changed_date_from/to`) escaped

---

### T1.4: Add JSON Pointer escaping for field names

RFC 6901: `~` → `~0`, `/` → `~1`.

**File**: `src/azure/work_items.rs` — modify

Add a helper function and use it for all JSON Patch `path` fields:

```rust
fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}
```

Apply to all `format!("/fields/{}", ...)` and `format!("/multilineFieldsFormat/{}", ...)` calls in `create_work_item` and `update_work_item`:

```rust
// create_work_item — multiline format fields:
path: format!("/multilineFieldsFormat/{}", escape_json_pointer_token(field)),

// create_work_item — field values:
path: format!("/fields/{}", escape_json_pointer_token(k)),

// update_work_item — multiline format fields:
path: format!("/multilineFieldsFormat/{}", escape_json_pointer_token(field)),

// update_work_item — field values:
path: format!("/fields/{}", escape_json_pointer_token(field)),
```

**DoD**:

- [ ] `escape_json_pointer_token` helper added
- [ ] All 4 JSON Patch path constructions use the helper

---

## US2: Security — CSV Formula Injection Mitigation

Prevents spreadsheet applications from interpreting cell values as formulas.

### Acceptance Criteria

- [ ] String values starting with `=`, `+`, `-`, `@` are prefixed with a single quote in CSV output

---

### T2.1: Add formula-safe escaping to CSV utilities

**File**: `src/mcp/tools/support/work_items_to_csv.rs` — modify

Replace the inline newline/tab/cr escaping in the string-value branch with a call to `sanitize_csv_value(s)` (imported from the shared `csv_sanitize` module created below).

**File**: `src/mcp/tools/support/board_columns_to_csv.rs` — modify

Apply the same sanitization to `column.name` and `column.column_type` values. Extract `sanitize_csv_value` into a shared location or duplicate in this file (prefer shared).

To share the function: add it to `src/mcp/tools/support/mod.rs` as a `pub(crate)` function in a new file `csv_sanitize.rs`, or keep it inline in each CSV file since the function is small.

Decision: add `csv_sanitize.rs` to support module with the shared function, import in both CSV files.

**File**: `src/mcp/tools/support/csv_sanitize.rs` — create

```rust
pub fn sanitize_csv_value(s: &str) -> String {
    let escaped = s
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "");
    if escaped.starts_with('=')
        || escaped.starts_with('+')
        || escaped.starts_with('-')
        || escaped.starts_with('@')
    {
        format!("'{}", escaped)
    } else {
        escaped
    }
}
```

**File**: `src/mcp/tools/support/mod.rs` — modify

Add `mod csv_sanitize;` and `pub use csv_sanitize::sanitize_csv_value;`.

**DoD**:

- [ ] `sanitize_csv_value` function created in `csv_sanitize.rs`
- [ ] `work_items_to_csv.rs` uses `sanitize_csv_value` for string values
- [ ] `board_columns_to_csv.rs` uses `sanitize_csv_value` for `name` and `column_type`

---

## US3: Code Quality — Duplicate Types, Error Handling, Validation

### Acceptance Criteria

- [ ] No duplicate board types between `models.rs` and `boards.rs`
- [ ] Zero `unwrap()` on `serde_json::to_value` or `compact_llm::to_compact_string` in MCP tool files
- [ ] All required string identifier fields validate non-empty

---

### T3.1: Remove duplicate board types from `models.rs`

**File**: `src/azure/models.rs` — modify

Remove `BoardListResponse`, `BoardReference`, `BoardColumn`, and `Board` structs. These are unused duplicates of types in `boards.rs`. Keep: `WorkItemListResponse`, `WorkItem`, `CommentListResponse`, `Comment`, `WiqlQuery`, `WiqlResponse`, `WorkItemReference`.

Before removing, verify no file imports these types from `models.rs` (only from `boards.rs`).

**DoD**:

- [ ] `BoardListResponse`, `BoardReference`, `BoardColumn`, `Board` removed from `models.rs`
- [ ] No compile errors after removal (confirms they were unused)

---

### T3.2: Replace `unwrap()` with proper error handling in MCP tools

Replace every `.unwrap()` on `serde_json::to_value(...)` and `compact_llm::to_compact_string(...)` with `.map_err(|e| McpError { code: ErrorCode(-32000), message: format!("Failed to serialize response: {}", e).into(), data: None })?`.

**Files to modify** (each follows the same pattern):

| File | unwrap() calls to replace |
|------|--------------------------|
| `src/mcp/tools/teams/boards/get_team_board.rs` | `compact_llm::to_compact_string(&board).unwrap()` |
| `src/mcp/tools/teams/boards/list_team_boards.rs` | `compact_llm::to_compact_string(&board_names).unwrap()` |
| `src/mcp/tools/teams/boards/list_board_rows.rs` | `compact_llm::to_compact_string(&row_names).unwrap()` |
| `src/mcp/tools/work_item_types/list_work_item_types.rs` | `compact_llm::to_compact_string(&type_names).unwrap()` |
| `src/mcp/tools/work_items/create_work_item.rs` | `serde_json::to_value(...).unwrap()` and `compact_llm::to_compact_string(...).unwrap()` |
| `src/mcp/tools/work_items/update_work_item.rs` | `serde_json::to_value(...).unwrap()` and `compact_llm::to_compact_string(...).unwrap()` |
| `src/mcp/tools/work_items/get_work_item.rs` | `serde_json::to_value(...).unwrap()` |
| `src/mcp/tools/work_items/get_work_items.rs` | `serde_json::to_value(...).unwrap()` |
| `src/mcp/tools/work_items/link_work_items.rs` | `compact_llm::to_compact_string(&result).unwrap()` |
| `src/mcp/tools/work_items/query_work_items.rs` | `serde_json::to_value(...).unwrap()` |
| `src/mcp/tools/work_items/query_work_items_by_wiql.rs` | `serde_json::to_value(...).unwrap()` |

Reference pattern (from `add_comment.rs` which already handles it correctly):

```rust
let output = compact_llm::to_compact_string(&some_value)
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to serialize response: {}", e).into(),
        data: None,
    })?;
```

For `serde_json::to_value`:

```rust
let mut json_value = serde_json::to_value(&work_items)
    .map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to serialize response: {}", e).into(),
        data: None,
    })?;
```

Files that need BOTH `serde_json::to_value` AND `compact_llm::to_compact_string` fixed: `create_work_item.rs`, `update_work_item.rs`.

Files that need ONLY `serde_json::to_value` fixed: `get_work_item.rs`, `get_work_items.rs`, `query_work_items.rs`, `query_work_items_by_wiql.rs`.

Files that need ONLY `compact_llm::to_compact_string` fixed: `get_team_board.rs`, `list_team_boards.rs`, `list_board_rows.rs`, `list_work_item_types.rs`, `link_work_items.rs`.

**DoD**:

- [ ] Zero `unwrap()` on serialization calls in all 11 listed tool files
- [ ] All use `.map_err(...)` with `ErrorCode(-32000)` and descriptive message

---

### T3.3: Add `deserialize_non_empty_string` to missing tool parameter fields

Add `#[serde(deserialize_with = "deserialize_non_empty_string")]` to these fields:

| File | Field(s) |
|------|----------|
| `src/mcp/tools/teams/get_team.rs` | `team_id` |
| `src/mcp/tools/teams/list_team_members.rs` | `team_id` |
| `src/mcp/tools/teams/get_team_current_iteration.rs` | `team_id` |
| `src/mcp/tools/teams/boards/list_team_boards.rs` | `team_id` |
| `src/mcp/tools/teams/boards/get_team_board.rs` | `team_id`, `board_id` |
| `src/mcp/tools/teams/boards/list_board_columns.rs` | `team_id`, `board_id` |
| `src/mcp/tools/teams/boards/list_board_rows.rs` | `team_id`, `board_id` |
| `src/mcp/tools/work_items/create_work_item.rs` | `work_item_type`, `title` |
| `src/mcp/tools/work_items/link_work_items.rs` | `link_type` |

Each field gets the `#[serde(deserialize_with = "deserialize_non_empty_string")]` attribute. Ensure `deserialize_non_empty_string` is imported in each file (some already import it for `organization`/`project`).

**DoD**:

- [ ] All 9 files updated with validation on all listed fields
- [ ] Each file imports `deserialize_non_empty_string`

---

## US4: Performance Improvements

### Acceptance Criteria

- [ ] Comment fetching runs concurrently (bounded to 10 parallel requests)
- [ ] Recursion depth limited to 64 in JSON processing functions
- [ ] HTTP server limits concurrent connections to 256 with 60s timeout
- [ ] `BoardDetail::get_work_item_types` uses `HashSet` for deduplication

---

### T4.1: Parallelize comment fetching with bounded concurrency

**File**: `Cargo.toml` — modify

Add `futures` dependency:

```toml
futures = "0.3"
```

**File**: `src/azure/work_items.rs` — modify

Replace sequential comment loop with bounded concurrent fetching:

```rust
use futures::future::join_all;

const COMMENT_FETCH_CONCURRENCY: usize = 10;
```

Replace the sequential comment loop in `get_work_items`:

```rust
if let Some(n) = include_latest_n_comments {
    let ids: Vec<(usize, u32)> = all_work_items
        .iter()
        .enumerate()
        .map(|(i, wi)| (i, wi.id))
        .collect();

    for chunk in ids.chunks(COMMENT_FETCH_CONCURRENCY) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|&(_, id)| get_comments(client, organization, project, id, n))
            .collect();

        let results = join_all(futures).await;

        for (&(i, _), result) in chunk.iter().zip(results) {
            all_work_items[i].comments = Some(result?);
        }
    }
}
```

**DoD**:

- [ ] `futures` added to `Cargo.toml`
- [ ] Comment fetching runs in chunks of 10 concurrent requests
- [ ] Errors propagate correctly from concurrent fetches

---

### T4.2: Add recursion depth limits to JSON processing

**File**: `src/mcp/tools/support/simplify_work_item_json.rs` — modify

Add a depth parameter with a public wrapper:

```rust
const MAX_RECURSION_DEPTH: usize = 64;

pub fn simplify_work_item_json(value: &mut Value) {
    simplify_work_item_json_inner(value, 0);
}

fn simplify_work_item_json_inner(value: &mut Value, depth: usize) {
    if depth > MAX_RECURSION_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            // ... existing logic, but recursive calls use:
            // simplify_work_item_json_inner(v, depth + 1);
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                simplify_work_item_json_inner(item, depth + 1);
            }
        }
        _ => {}
    }
}
```

**File**: `src/compact_llm.rs` — modify

Same pattern for `write_compact_value`:

```rust
const MAX_RECURSION_DEPTH: usize = 64;

fn write_compact_value(value: &serde_json::Value, output: &mut String) {
    write_compact_value_inner(value, output, 0);
}

fn write_compact_value_inner(value: &serde_json::Value, output: &mut String, depth: usize) {
    if depth > MAX_RECURSION_DEPTH {
        output.push_str("...");
        return;
    }
    match value {
        // ... existing logic, but recursive calls use depth + 1
    }
}
```

**DoD**:

- [ ] `simplify_work_item_json` stops recursion at depth 64
- [ ] `write_compact_value` stops recursion at depth 64, outputs `...` for truncated nodes
- [ ] Public API signature unchanged (depth is internal)

---

### T4.3: Add HTTP connection limits and timeouts

**File**: `src/server/http.rs` — modify

Add connection semaphore (256) and per-connection timeout (60s). Refactor to accept `TcpListener` for testability (used by US8).

```rust
use crate::mcp::server::AzureMcpServer;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const MAX_CONNECTIONS: usize = 256;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn run_server(
    server: AzureMcpServer,
    listener: tokio::net::TcpListener,
) -> std::io::Result<()> {
    let service = TowerToHyperService::new(StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    ));

    let semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));

    loop {
        let (stream, _) = listener.accept().await?;
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(e) => {
                log::error!("Failed to acquire connection permit: {:?}", e);
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let service = service.clone();

        tokio::spawn(async move {
            let result = tokio::time::timeout(
                CONNECTION_TIMEOUT,
                Builder::new(TokioExecutor::default())
                    .serve_connection(io, service),
            )
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => log::error!("Error serving connection: {:?}", err),
                Err(_) => log::warn!("Connection timed out after {:?}", CONNECTION_TIMEOUT),
            }

            drop(permit);
        });
    }
}
```

**File**: `src/main.rs` — modify

Update to create listener and pass to `run_server`:

```rust
if args.server {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    log::info!("Starting web server on {}", listener.local_addr()?);
    http::run_server(mcp_server, listener).await?;
}
```

**DoD**:

- [ ] `run_server` accepts `TcpListener` instead of `port`
- [ ] Semaphore limits concurrent connections to 256
- [ ] Per-connection timeout of 60 seconds
- [ ] `main.rs` creates listener and passes it
- [ ] Logging uses `log::error!`/`log::warn!` instead of `eprintln!`

---

### T4.4: Use `HashSet` in `BoardDetail::get_work_item_types`

**File**: `src/azure/boards.rs` — modify

```rust
use std::collections::HashSet;

impl BoardDetail {
    pub fn get_work_item_types(&self) -> Vec<String> {
        let mut types = HashSet::new();

        if let Some(mappings) = &self.allowed_mappings
            && let Some(obj) = mappings.as_object()
        {
            for (_column_type, type_mappings) in obj {
                if let Some(type_obj) = type_mappings.as_object() {
                    for (work_item_type, _states) in type_obj {
                        types.insert(work_item_type.clone());
                    }
                }
            }
        }

        types.into_iter().collect()
    }
}
```

**DoD**:

- [ ] Deduplication uses `HashSet` instead of `Vec::contains`

---

## US5: Infrastructure — Docker, CI/CD

### Acceptance Criteria

- [ ] Docker runtime image runs as non-root user
- [ ] CI runs clippy and cargo-audit
- [ ] CD packages Linux aarch64 binary in release

---

### T5.1: Add non-root user to Dockerfile

**File**: `Dockerfile` — modify

Add `RUN adduser -D -g '' appuser` before the existing `WORKDIR /app` line (do NOT duplicate `WORKDIR /app` — it already exists). Add `USER appuser` after the `COPY --from=builder` line and before `ENTRYPOINT`:

```dockerfile
RUN adduser -D -g '' appuser

WORKDIR /app

COPY --from=builder /app/target/release/mcp-for-azure-devops-boards /usr/local/bin/mcp-for-azure-devops-boards

USER appuser

ENTRYPOINT ["mcp-for-azure-devops-boards"]
```

**DoD**:

- [ ] `appuser` created in runtime stage
- [ ] `WORKDIR /app` NOT duplicated (already exists)
- [ ] `USER appuser` directive set before `ENTRYPOINT`

---

### T5.2: Add clippy and cargo-audit to CI

**File**: `.github/workflows/ci-pr-build-and-test.yml` — modify

Add `components: clippy` to the existing `Setup Rust` step in `build-and-test` (matching the pattern used by the formatting job with `rustfmt`):

```yaml
      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          components: clippy
```

Add clippy step to `build-and-test` job (after Build, before Test):

```yaml
      - name: Clippy
        run: cargo clippy --features test-support --locked -- -D warnings
```

Add a new job for security audit:

```yaml
  security-audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install cargo-audit
        run: cargo install cargo-audit --locked

      - name: Security audit
        run: cargo audit
```

**DoD**:

- [ ] Clippy runs on all matrix platforms with `-D warnings`
- [ ] `cargo audit` runs in a separate job

---

### T5.3: Add Linux aarch64 to CD release

**File**: `.github/workflows/cd-tag-build-and-release.yml` — modify

Add a "Prepare Linux tar.gz" step in the `release` job, between the existing macOS and Windows steps:

```yaml
      - name: Prepare Linux tar.gz
        run: |
          set -eux
          TAG="${GITHUB_REF_NAME}"
          LINUX_DIR="dist/mcp-for-azure-devops-boards-ubuntu-24.04-aarch64"
          cp "${LINUX_DIR}/mcp-for-azure-devops-boards-ubuntu-24.04-aarch64" mcp-for-azure-devops-boards
          tar czf "mcp-for-azure-devops-boards-${TAG}-linux-aarch64.tar.gz" mcp-for-azure-devops-boards
          rm mcp-for-azure-devops-boards
          ls -l "mcp-for-azure-devops-boards-${TAG}-linux-aarch64.tar.gz"
```

Add the Linux archive to the release `files:` list:

```yaml
          files: |
            mcp-for-azure-devops-boards-${{ github.ref_name }}-linux-aarch64.tar.gz
            mcp-for-azure-devops-boards-${{ github.ref_name }}-macos-aarch64.tar.gz
            mcp-for-azure-devops-boards-${{ github.ref_name }}-windows-x86_64.zip
```

Also modify the existing macOS step to clean up the intermediate file after tarring (prevents name collision with Linux step):

```yaml
      - name: Prepare macOS tar.gz
        run: |
          set -eux
          TAG="${GITHUB_REF_NAME}"
          MAC_DIR="dist/mcp-for-azure-devops-boards-macos-aarch64"
          cp "${MAC_DIR}/mcp-for-azure-devops-boards-macos-aarch64" mcp-for-azure-devops-boards
          tar czf "mcp-for-azure-devops-boards-${TAG}-macos-aarch64.tar.gz" mcp-for-azure-devops-boards
          rm mcp-for-azure-devops-boards
          ls -l "mcp-for-azure-devops-boards-${TAG}-macos-aarch64.tar.gz"
```

**DoD**:

- [ ] Linux aarch64 tar.gz prepared in release job
- [ ] Linux archive included in `files:` list
- [ ] No file name collisions between Linux and macOS preparation steps

---

## US6: Test Coverage — compact_llm

### Acceptance Criteria

- [ ] Tests cover empty arrays, empty objects, Unicode, control characters, deeply nested values, long strings

---

### T6.1: Add missing test cases to `compact_llm.rs`

**File**: `src/compact_llm.rs` (`#[cfg(test)] mod tests`) — modify

| Test | Verifies |
|------|----------|
| `test_empty_object` | `{}` serializes to `{}` |
| `test_empty_array` | `[]` serializes to `[]` |
| `test_unicode_strings` | Unicode chars (CJK, emoji, accented) pass through unchanged |
| `test_control_characters` | Control chars other than `\n`/`\r` (e.g. `\t`, `\0`) pass through (not escaped) |
| `test_deeply_nested_object` | 64 levels of nesting serialize correctly |
| `test_beyond_max_depth` | 65+ levels of nesting produce `...` for truncated nodes |
| `test_long_string` | String of 10000 chars serializes without issue |
| `test_empty_string_value` | Empty string produces empty output between delimiters |
| `test_mixed_array` | Array with null, bool, number, string, object elements |

**DoD**:

- [ ] All 9 test cases added and passing

---

## US7: Test Coverage — Error Paths & Content Verification

### Acceptance Criteria

- [ ] Every MCP tool has at least one error-propagation test
- [ ] Every MCP tool has at least one content-verification test

---

### T7.1: Error propagation tests for all tools

Add `AzureError::ApiError` error tests for every tool that calls the API. Existing test: `test_get_work_item_api_error_propagates` (keep as-is).

**File**: `tests/test_tools_organizations.rs` — modify

| Test | Verifies | Setup notes |
|------|----------|-------------|
| `test_list_organizations_get_profile_error_propagates` | `expect_get_profile` returns `Err(ApiError(...))` → tool returns `Err` | Tests first API call failure path |
| `test_list_organizations_list_orgs_error_propagates` | `expect_get_profile` succeeds, `expect_list_organizations` returns `Err(ApiError(...))` → tool returns `Err` | Tests second API call failure path |
| `test_get_current_user_api_error_propagates` | `expect_get_profile` returns `Err(ApiError(...))` → tool returns `Err` | |

**File**: `tests/test_tools_projects.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_projects_api_error_propagates` | `expect_list_projects` returns `Err(ApiError(...))` → tool returns `Err` |

**File**: `tests/test_tools_teams.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_teams_api_error_propagates` | `expect_list_teams` → `Err` |
| `test_get_team_api_error_propagates` | `expect_get_team` → `Err` |
| `test_list_team_members_api_error_propagates` | `expect_list_team_members` → `Err` |
| `test_get_team_current_iteration_api_error_propagates` | `expect_get_team_current_iteration` → `Err` |

**File**: `tests/test_tools_boards.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_team_boards_api_error_propagates` | `expect_list_boards` → `Err` |
| `test_get_team_board_api_error_propagates` | `expect_get_board` → `Err` |
| `test_list_board_columns_api_error_propagates` | `expect_list_board_columns` → `Err` |
| `test_list_board_rows_api_error_propagates` | `expect_list_board_rows` → `Err` |

**File**: `tests/test_tools_tags.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_tags_api_error_propagates` | `expect_list_tags` → `Err` |

**File**: `tests/test_tools_work_item_types.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_work_item_types_api_error_propagates` | `expect_list_work_item_types` → `Err` |

**File**: `tests/test_tools_classification_nodes.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_area_paths_api_error_propagates` | `expect_list_area_paths` → `Err` |
| `test_list_iteration_paths_api_error_propagates` | `expect_list_iteration_paths` → `Err` |

**File**: `tests/test_tools_work_items.rs` — modify

Existing: `test_get_work_item_api_error_propagates`.

| Test | Verifies |
|------|----------|
| `test_get_work_items_api_error_propagates` | `expect_get_work_items` → `Err` |
| `test_create_work_item_api_error_propagates` | `expect_create_work_item` → `Err` |
| `test_update_work_item_api_error_propagates` | `expect_update_work_item` → `Err` |
| `test_query_work_items_api_error_propagates` | `expect_query_work_items` → `Err` |
| `test_query_work_items_by_wiql_api_error_propagates` | `expect_query_work_items` → `Err` |
| `test_link_work_items_api_error_propagates` | `expect_link_work_items` → `Err` |
| `test_add_comment_api_error_propagates` | `expect_add_comment` → `Err` |
| `test_update_comment_api_error_propagates` | `expect_update_comment` → `Err` |

**Setup pattern**: `mock.expect_<method>().returning(|..| Err(AzureError::ApiError("test error".to_string())))`, then `assert!(result.is_err())`.

**DoD**:

- [ ] 25 error-propagation tests total (1 existing + 24 new)
- [ ] All tests pass

---

### T7.2: Content verification tests for all tools

Use `extract_text_from_result` to check actual content structure. Strip `UNTRUSTED_CONTENT_WARNING` prefix before asserting.

**File**: `tests/test_tools_organizations.rs` — modify

| Test | Verifies | Setup notes |
|------|----------|-------------|
| `test_list_organizations_returns_org_names` | Output contains comma-separated org names | Mock 2 organizations, verify both names in output |
| `test_get_current_user_returns_csv_with_profile` | Output contains user CSV with `display_name`, `email_address` columns | Mock profile, verify CSV header and values |

**File**: `tests/test_tools_projects.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_projects_returns_project_names` | Output contains comma-separated project names |

**File**: `tests/test_tools_teams.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_teams_returns_team_names` | Output contains comma-separated team names |
| `test_get_team_returns_team_details` | Output contains team name, id, description |
| `test_list_team_members_returns_csv` | Output has CSV with display_name, unique_name, id |
| `test_get_team_current_iteration_no_iteration` | Returns "No current iteration found" when mock returns `None` |

**File**: `tests/test_tools_boards.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_team_boards_returns_board_names` | Output contains board name list |
| `test_get_team_board_returns_compact_json` | Output contains board id and name in compact format |
| `test_list_board_columns_returns_csv` | Output has CSV with name, item_limit, is_split, column_type headers |
| `test_list_board_rows_returns_row_names` | Output contains row name list |

**File**: `tests/test_tools_tags.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_tags_returns_tag_names` | Output contains comma-separated tag names |

**File**: `tests/test_tools_work_item_types.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_work_item_types_returns_type_names` | Output contains work item type names |

**File**: `tests/test_tools_classification_nodes.rs` — modify

| Test | Verifies |
|------|----------|
| `test_list_area_paths_returns_paths` | Output contains comma-separated paths |
| `test_list_iteration_paths_returns_paths` | Output contains comma-separated paths |

**File**: `tests/test_tools_work_items.rs` — modify

| Test | Verifies | Setup notes |
|------|----------|-------------|
| `test_get_work_item_returns_csv_with_fields` | CSV output has id, Type, Title columns | Mock with known field values |
| `test_get_work_item_not_found_returns_message` | Returns "Work item not found" when mock returns `Ok(None)` | |
| `test_get_work_items_returns_csv` | CSV output has multiple rows | Mock 2 work items |
| `test_get_work_items_empty_ids_returns_message` | Returns "No work items found" for empty ids | |
| `test_create_work_item_returns_compact_json` | Output has work item id in compact format | |
| `test_update_work_item_returns_compact_json` | Output has work item id in compact format | |
| `test_query_work_items_returns_csv` | CSV output from query results | |
| `test_query_work_items_empty_returns_message` | Returns "No work items found" when no results | |
| `test_query_work_items_by_wiql_returns_csv` | CSV output from WIQL query | |
| `test_link_work_items_returns_compact_json` | Output has link result in compact format | |
| `test_add_comment_returns_compact_json` | Output has comment data | |
| `test_update_comment_returns_compact_json` | Output has updated comment data | |

**DoD**:

- [ ] All content verification tests pass
- [ ] Tests verify actual data structure (CSV headers, field presence, expected values)
- [ ] No duplicate coverage with existing tests

---

## US8: Test Coverage — HTTP Server & CLI

### Acceptance Criteria

- [ ] HTTP server integration test that starts server, makes request, verifies response
- [ ] CLI argument parsing tests for all flags

---

### T8.1: Add HTTP server integration test

**File**: `tests/test_server_http.rs` — create

Test starts a real HTTP server on port 0 (OS-assigned), sends an MCP initialize request, verifies response.

| Test | Verifies | Setup notes |
|------|----------|-------------|
| `test_http_server_accepts_connection` | Server binds, accepts TCP connection, responds to HTTP POST | Bind to `127.0.0.1:0`, spawn server task, use `reqwest::Client` to POST to `/mcp`, verify 2xx response |
| `test_http_server_rejects_invalid_method` | GET to `/mcp` returns appropriate error | Same setup, send GET instead of POST |

Setup: Create `AzureMcpServer` with `MockAzureDevOpsApi`, bind `TcpListener` to `127.0.0.1:0`, get `local_addr()`, spawn `run_server` in background task, use `reqwest` to make requests, abort server task on cleanup.

**DoD**:

- [ ] Tests start actual HTTP server and make real HTTP requests
- [ ] Tests clean up server task

---

### T8.2: Add CLI argument parsing tests

**File**: `src/main.rs` — modify

Extract `Args` struct to be `pub` (or `pub(crate)`) so it can be tested. Since integration tests can't access private items in `main.rs`, add unit tests inline.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_default_args() { ... }
    #[test]
    fn test_server_flag() { ... }
    #[test]
    fn test_custom_port() { ... }
    #[test]
    fn test_server_with_port() { ... }
}
```

| Test | Verifies |
|------|----------|
| `test_default_args` | No flags → `server = false`, `port = 3000` |
| `test_server_flag` | `--server` → `server = true`, `port = 3000` |
| `test_custom_port` | `--port 8080` → `port = 8080` (server still false) |
| `test_server_with_port` | `--server --port 8080` → both set correctly |

**DoD**:

- [ ] 4 CLI parsing tests pass
- [ ] All tests use `Args::try_parse_from(...)` for isolated parsing

---

## US9: Final Verification

### Acceptance Criteria

- [ ] `make all` passes (fmt → lint → test → build)
- [ ] No warnings from build or clippy
- [ ] All tests pass
- [ ] No TODOs or commented-out code introduced

---

### T9.1: Run full quality gate and verify from ground up

1. Run `make all` (fmt → lint → test → build)
2. Review every changed file against this plan
3. Verify:
   - [ ] US1: URL encoding applied everywhere listed
   - [ ] US1: WIQL escaping applied to project + 6 date fields
   - [ ] US1: JSON Pointer escaping applied to all patch paths
   - [ ] US2: CSV formula sanitization applied in both CSV files
   - [ ] US3: No duplicate board types in models.rs
   - [ ] US3: Zero unwrap() on serialization in tool files
   - [ ] US3: All listed fields have non-empty validation
   - [ ] US4: Comment fetching is concurrent with limit 10
   - [ ] US4: Recursion depth capped at 64 in both functions
   - [ ] US4: HTTP server has semaphore (256) and timeout (60s)
   - [ ] US4: HashSet used for work item type dedup
   - [ ] US5: Dockerfile has non-root user
   - [ ] US5: CI runs clippy and cargo-audit
   - [ ] US5: CD includes Linux aarch64 archive
   - [ ] US6: All 9 compact_llm tests pass
   - [ ] US7: 25 error-path tests + ~25 content-verification tests pass
   - [ ] US8: HTTP server and CLI tests pass
   - [ ] Zero lint warnings, zero build warnings, all tests green

**DoD**:

- [ ] `make all` succeeds with zero warnings
- [ ] Every checkbox in this plan is checked

---

## Review Findings

### Plan Reviewer Audit (post-creation)

| ID | Severity | Finding | Resolution |
|----|----------|---------|------------|
| W-1 | WARNING | T7.1 DoD test count was 22, actual is 25 (1 existing + 24 new including W-6 addition) | Fixed: DoD updated to 25 |
| W-2 | WARNING | T2.1 had inconsistent inline `sanitize_csv_value` with dead code checks for `\t`/`\r` | Fixed: removed inline version, kept only shared `csv_sanitize.rs` |
| W-3 | WARNING | T5.1 Dockerfile code block included duplicate `WORKDIR /app` | Fixed: clarified `WORKDIR /app` already exists |
| W-4 | WARNING | T5.3 macOS step cleanup mentioned in prose but not as explicit action | Fixed: added explicit macOS step code with `rm` |
| W-5 | WARNING | T5.2 CI setup step missing `components: clippy` | Fixed: added `components: clippy` to setup step |
| W-6 | WARNING | `list_organizations` error test only covered `get_profile` failure, not `list_organizations` API call failure | Fixed: added second test `test_list_organizations_list_orgs_error_propagates` |
| I-1 | INFO | Error message pattern uses `"Failed to serialize response: {}"` vs existing `e.to_string()` | Kept plan pattern — more descriptive context |
| I-2 | INFO | Binary size tradeoff from opt-level change | Resolved: P5 dropped from plan (binary too large) |
| I-3 | INFO | `BoardColumn` type difference between models.rs and boards.rs | Noted: plan correctly removes unused models.rs versions |
| I-4 | INFO | Labels in boards.rs code block may confuse implementer | Noted: cosmetic only, function names are in boards.rs |
| I-5 | INFO | Semaphore `acquire_owned` error uses silent `continue` | Fixed: added `log::error!` before `continue` |
| I-6 | INFO | HTTP server test requires explicit task abort for cleanup | Noted: setup description already covers this |
