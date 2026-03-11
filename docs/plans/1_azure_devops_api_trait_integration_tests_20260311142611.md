<!-- SACRED DOCUMENT — DO NOT MODIFY except for checkmarks ([ ] → [x]) and review findings. -->
<!-- You MUST NEVER alter, revert, or delete files outside the scope of this plan. -->
<!-- Plans in docs/plans/ are PERMANENT artifacts. There are ZERO exceptions. -->

# Define AzureDevOpsApi trait and add integration tests for anti-prompt-injection

Tools currently take `&AzureDevOpsClient` directly, making mocking impossible. This plan introduces an `AzureDevOpsApi` trait so integration tests can verify every tool prepends the anti-prompt-injection warning.

**Deliberate deviation**: The trait has 23 methods (exceeds the 1–3 guideline). Justified: it represents a single external system boundary (Azure DevOps); splitting would add indirection without testability benefit.

---

## US1: Define AzureDevOpsApi trait

Establishes the mockable API boundary so tools can be tested without external services.

- [ ] `list_team_members` normalized from `impl AzureDevOpsClient` method to standalone fn in `teams.rs`
- [ ] `create_work_item`/`update_work_item` in `work_items.rs` accept owned types: `&[(String, Value)]`, `&[(String, String)]`
- [ ] `AzureDevOpsApi` trait in `src/azure/api_trait.rs` with 23 async methods and `#[cfg_attr(feature = "test-support", mockall::automock)]`
- [ ] `impl AzureDevOpsApi for AzureDevOpsClient` delegating to existing standalone functions
- [ ] `mockall` moved to optional dependency, `test-support` feature added to `Cargo.toml`
- [ ] Module registered in `src/azure/mod.rs`
- [ ] `UNTRUSTED_CONTENT_WARNING` exported from `src/mcp/tools/support/mod.rs`
- [ ] Makefile updated to pass `--features test-support` to test/lint targets

### Task 1.1: Normalize `list_team_members` in `src/azure/teams.rs` (modify)

Replace the `impl AzureDevOpsClient` block with a standalone function:

```diff
-impl AzureDevOpsClient {
-    pub async fn list_team_members(
-        &self,
-        organization: &str,
-        project: &str,
-        team_id: &str,
-    ) -> Result<Vec<TeamMember>, AzureError> {
-        let path = format!(
-            "projects/{}/teams/{}/members?api-version=7.1",
-            project, team_id
-        );
-        let response: TeamMembersResponse = self
-            .org_request(organization, Method::GET, &path, None::<&String>)
-            .await?;
-
-        Ok(response.value)
-    }
-}
+pub async fn list_team_members(
+    client: &AzureDevOpsClient,
+    organization: &str,
+    project: &str,
+    team_id: &str,
+) -> Result<Vec<TeamMember>, AzureError> {
+    let path = format!(
+        "projects/{}/teams/{}/members?api-version=7.1",
+        project, team_id
+    );
+    let response: TeamMembersResponse = client
+        .org_request(organization, Method::GET, &path, None::<&String>)
+        .await?;
+
+    Ok(response.value)
+}
```

### Task 1.2: Update slice params in `src/azure/work_items.rs` (modify)

Change `create_work_item` and `update_work_item` signatures from `&[(&str, Value)]`/`&[(&str, &str)]` to `&[(String, Value)]`/`&[(String, String)]`:

```diff
 pub async fn create_work_item(
     client: &AzureDevOpsClient,
     organization: &str,
     project: &str,
     work_item_type: &str,
-    fields: &[(&str, Value)],
-    multiline_fields_format: &[(&str, &str)],
+    fields: &[(String, Value)],
+    multiline_fields_format: &[(String, String)],
 ) -> Result<WorkItem, AzureError> {
```

```diff
 pub async fn update_work_item(
     client: &AzureDevOpsClient,
     organization: &str,
     project: &str,
     id: u32,
-    fields: &[(&str, Value)],
-    multiline_fields_format: &[(&str, &str)],
+    fields: &[(String, Value)],
+    multiline_fields_format: &[(String, String)],
 ) -> Result<WorkItem, AzureError> {
```

Function bodies unchanged — `format!("/fields/{}", k)` and `format.to_string()` work identically with `String` params.

### Task 1.3: Create `src/azure/api_trait.rs` (create)

```rust
use async_trait::async_trait;
use serde_json::Value;

use crate::azure::boards::{BoardColumn, BoardDetail, BoardRow, BoardSummary, Team, WorkItemType};
use crate::azure::classification_nodes::ClassificationNode;
use crate::azure::client::{AzureDevOpsClient, AzureError};
use crate::azure::iterations::TeamSettingsIteration;
use crate::azure::models::WorkItem;
use crate::azure::organizations::{Organization, Profile};
use crate::azure::projects::Project;
use crate::azure::tags::TagDefinition;
use crate::azure::teams::TeamMember;
use crate::azure::{
    boards, classification_nodes, iterations, organizations, projects, tags, teams, work_items,
};

#[cfg_attr(feature = "test-support", mockall::automock)]
#[async_trait]
pub trait AzureDevOpsApi {
    async fn get_profile(&self) -> Result<Profile, AzureError>;
    async fn list_organizations(&self, member_id: &str) -> Result<Vec<Organization>, AzureError>;
    async fn list_projects(&self, organization: &str) -> Result<Vec<Project>, AzureError>;
    async fn list_teams(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<Team>, AzureError>;
    async fn get_team(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Team, AzureError>;
    async fn list_team_members(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Vec<TeamMember>, AzureError>;
    async fn list_work_item_types(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<WorkItemType>, AzureError>;
    async fn list_boards(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Vec<BoardSummary>, AzureError>;
    async fn get_board(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<BoardDetail, AzureError>;
    async fn list_board_columns(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<Vec<BoardColumn>, AzureError>;
    async fn list_board_rows(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<Vec<BoardRow>, AzureError>;
    async fn list_tags(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<TagDefinition>, AzureError>;
    async fn get_team_current_iteration(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Option<TeamSettingsIteration>, AzureError>;
    async fn list_area_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<&str>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError>;
    async fn list_iteration_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<&str>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError>;
    async fn get_work_item(
        &self,
        organization: &str,
        project: &str,
        id: u32,
        include_latest_n_comments: Option<i32>,
    ) -> Result<Option<WorkItem>, AzureError>;
    async fn get_work_items(
        &self,
        organization: &str,
        project: &str,
        ids: &[u32],
        include_latest_n_comments: Option<i32>,
    ) -> Result<Vec<WorkItem>, AzureError>;
    async fn create_work_item(
        &self,
        organization: &str,
        project: &str,
        work_item_type: &str,
        fields: &[(String, Value)],
        multiline_fields_format: &[(String, String)],
    ) -> Result<WorkItem, AzureError>;
    async fn update_work_item(
        &self,
        organization: &str,
        project: &str,
        id: u32,
        fields: &[(String, Value)],
        multiline_fields_format: &[(String, String)],
    ) -> Result<WorkItem, AzureError>;
    async fn add_comment(
        &self,
        organization: &str,
        project: &str,
        work_item_id: u32,
        text: &str,
        format: &str,
    ) -> Result<Value, AzureError>;
    async fn update_comment(
        &self,
        organization: &str,
        project: &str,
        work_item_id: u32,
        comment_id: u32,
        text: &str,
        format: &str,
    ) -> Result<Value, AzureError>;
    async fn link_work_items(
        &self,
        organization: &str,
        project: &str,
        source_id: u32,
        target_id: u32,
        link_type: &str,
    ) -> Result<Value, AzureError>;
    async fn query_work_items(
        &self,
        organization: &str,
        project: &str,
        query: &str,
        include_latest_n_comments: Option<i32>,
    ) -> Result<Vec<WorkItem>, AzureError>;
}

#[async_trait]
impl AzureDevOpsApi for AzureDevOpsClient {
    async fn get_profile(&self) -> Result<Profile, AzureError> {
        organizations::get_profile(self).await
    }
    async fn list_organizations(&self, member_id: &str) -> Result<Vec<Organization>, AzureError> {
        organizations::list_organizations(self, member_id).await
    }
    async fn list_projects(&self, organization: &str) -> Result<Vec<Project>, AzureError> {
        projects::list_projects(self, organization).await
    }
    async fn list_teams(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<Team>, AzureError> {
        boards::list_teams(self, organization, project).await
    }
    async fn get_team(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Team, AzureError> {
        boards::get_team(self, organization, project, team_id).await
    }
    async fn list_team_members(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Vec<TeamMember>, AzureError> {
        teams::list_team_members(self, organization, project, team_id).await
    }
    async fn list_work_item_types(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<WorkItemType>, AzureError> {
        boards::list_work_item_types(self, organization, project).await
    }
    async fn list_boards(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Vec<BoardSummary>, AzureError> {
        boards::list_boards(self, organization, project, team_id).await
    }
    async fn get_board(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<BoardDetail, AzureError> {
        boards::get_board(self, organization, project, team_id, board_id).await
    }
    async fn list_board_columns(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<Vec<BoardColumn>, AzureError> {
        boards::list_board_columns(self, organization, project, team_id, board_id).await
    }
    async fn list_board_rows(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        board_id: &str,
    ) -> Result<Vec<BoardRow>, AzureError> {
        boards::list_board_rows(self, organization, project, team_id, board_id).await
    }
    async fn list_tags(
        &self,
        organization: &str,
        project: &str,
    ) -> Result<Vec<TagDefinition>, AzureError> {
        tags::list_tags(self, organization, project).await
    }
    async fn get_team_current_iteration(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
    ) -> Result<Option<TeamSettingsIteration>, AzureError> {
        iterations::get_team_current_iteration(self, organization, project, team_id).await
    }
    async fn list_area_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<&str>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError> {
        classification_nodes::list_area_paths(self, organization, project, parent_path, depth).await
    }
    async fn list_iteration_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<&str>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError> {
        classification_nodes::list_iteration_paths(self, organization, project, parent_path, depth)
            .await
    }
    async fn get_work_item(
        &self,
        organization: &str,
        project: &str,
        id: u32,
        include_latest_n_comments: Option<i32>,
    ) -> Result<Option<WorkItem>, AzureError> {
        work_items::get_work_item(self, organization, project, id, include_latest_n_comments).await
    }
    async fn get_work_items(
        &self,
        organization: &str,
        project: &str,
        ids: &[u32],
        include_latest_n_comments: Option<i32>,
    ) -> Result<Vec<WorkItem>, AzureError> {
        work_items::get_work_items(self, organization, project, ids, include_latest_n_comments)
            .await
    }
    async fn create_work_item(
        &self,
        organization: &str,
        project: &str,
        work_item_type: &str,
        fields: &[(String, Value)],
        multiline_fields_format: &[(String, String)],
    ) -> Result<WorkItem, AzureError> {
        work_items::create_work_item(
            self,
            organization,
            project,
            work_item_type,
            fields,
            multiline_fields_format,
        )
        .await
    }
    async fn update_work_item(
        &self,
        organization: &str,
        project: &str,
        id: u32,
        fields: &[(String, Value)],
        multiline_fields_format: &[(String, String)],
    ) -> Result<WorkItem, AzureError> {
        work_items::update_work_item(self, organization, project, id, fields, multiline_fields_format)
            .await
    }
    async fn add_comment(
        &self,
        organization: &str,
        project: &str,
        work_item_id: u32,
        text: &str,
        format: &str,
    ) -> Result<Value, AzureError> {
        work_items::add_comment(self, organization, project, work_item_id, text, format).await
    }
    async fn update_comment(
        &self,
        organization: &str,
        project: &str,
        work_item_id: u32,
        comment_id: u32,
        text: &str,
        format: &str,
    ) -> Result<Value, AzureError> {
        work_items::update_comment(
            self,
            organization,
            project,
            work_item_id,
            comment_id,
            text,
            format,
        )
        .await
    }
    async fn link_work_items(
        &self,
        organization: &str,
        project: &str,
        source_id: u32,
        target_id: u32,
        link_type: &str,
    ) -> Result<Value, AzureError> {
        work_items::link_work_items(self, organization, project, source_id, target_id, link_type)
            .await
    }
    async fn query_work_items(
        &self,
        organization: &str,
        project: &str,
        query: &str,
        include_latest_n_comments: Option<i32>,
    ) -> Result<Vec<WorkItem>, AzureError> {
        work_items::query_work_items(self, organization, project, query, include_latest_n_comments)
            .await
    }
}
```

### Task 1.4: Update `Cargo.toml` (modify)

Move `mockall` from `[dev-dependencies]` to `[dependencies]` as optional, add `test-support` feature:

```diff
 [dependencies]
+mockall = { version = "0.12", optional = true }
 mcp-tools-codegen = { path = "mcp-tools-codegen" }
@@ dev-dependencies
 [dev-dependencies]
-mockall = "0.12"
+
+[features]
+test-support = ["mockall"]
```

### Task 1.5: Update `Makefile` (modify)

Pass `--features test-support` to test and lint targets so `MockAzureDevOpsApi` is compiled:

```diff
 test:
-	cargo test
+	cargo test --features test-support

 lint:
-	cargo clippy -- -D warnings
+	cargo clippy --features test-support -- -D warnings
```

### Task 1.6: Register module in `src/azure/mod.rs` (modify)

```diff
+pub mod api_trait;
 pub mod boards;
```

### Task 1.7: Export constant in `src/mcp/tools/support/mod.rs` (modify)

```diff
-pub use tool_text_success::tool_text_success;
+pub use tool_text_success::{tool_text_success, UNTRUSTED_CONTENT_WARNING};
```

### DoD
- [ ] Normalized `list_team_members` standalone fn compiles
- [ ] Owned-type signatures in `work_items.rs` compile
- [ ] `api_trait.rs` compiles with trait + impl
- [ ] `test-support` feature in `Cargo.toml` enables `mockall`
- [ ] `make test` and `make lint` pass `--features test-support`

---

## US2: Wire trait through stack

Replace concrete `AzureDevOpsClient` references with the trait throughout the tool stack so mocks can be injected.

- [ ] `AzureMcpServer.client` is `Arc<dyn AzureDevOpsApi + Send + Sync>`
- [ ] `build.rs` delegates via `&*self.client`
- [ ] All 24 tool functions accept `&(dyn AzureDevOpsApi + Send + Sync)`
- [ ] All API calls go through trait methods
- [ ] `main.rs` unchanged

### Task 2.1: Update `src/mcp/server.rs` (modify)

```diff
+use crate::azure::api_trait::AzureDevOpsApi;
 use crate::azure::client::AzureDevOpsClient;
 use rmcp::{
     handler::server::router::tool::ToolRouter,
@@ struct
 #[derive(Clone)]
 pub struct AzureMcpServer {
-    client: Arc<AzureDevOpsClient>,
+    client: Arc<dyn AzureDevOpsApi + Send + Sync>,
     tool_router: ToolRouter<Self>,
 }
```

### Task 2.2: Update `build.rs` (modify)

In `generate_tool_router_code`, change the delegation line:

```diff
-        "        {}(&self.client, args.0).await\n",
+        "        {}(&*self.client, args.0).await\n",
```

### Task 2.3: Update all 24 tool files (modify)

#### Transformation pattern (applied to every tool file)

**Imports**: Replace `use crate::azure::{client::AzureDevOpsClient, MODULE};` with `use crate::azure::api_trait::AzureDevOpsApi;`

**Signature**: Replace `client: &AzureDevOpsClient` with `client: &(dyn AzureDevOpsApi + Send + Sync)`

**API calls**: Replace `MODULE::function(client, ...)` with `client.method(...)`

#### Representative example: `src/mcp/tools/organizations/list_organizations.rs`

```diff
-use crate::azure::{client::AzureDevOpsClient, organizations};
+use crate::azure::api_trait::AzureDevOpsApi;
 use crate::mcp::tools::support::tool_text_success;

 use mcp_tools_codegen::mcp_tool;
 use rmcp::{
     ErrorData as McpError,
     model::{CallToolResult, ErrorCode},
     schemars::{self, JsonSchema},
     serde::Deserialize,
 };
@@ fn
 pub async fn list_organizations(
-    client: &AzureDevOpsClient,
+    client: &(dyn AzureDevOpsApi + Send + Sync),
     _args: ListOrganizationsArgs,
 ) -> Result<CallToolResult, McpError> {
     log::info!("Tool invoked: azdo_list_organizations");
-    let profile = organizations::get_profile(client)
+    let profile = client.get_profile()
         .await
         .map_err(|e| McpError { ... })?;
-    let orgs = organizations::list_organizations(client, &profile.id)
+    let orgs = client.list_organizations(&profile.id)
         .await
         .map_err(|e| McpError { ... })?;
```

#### Representative example: `src/mcp/tools/work_items/create_work_item.rs`

Additional changes beyond the standard pattern — owned types for field vectors:

```diff
-    let mut multiline_formats: Vec<(&str, &str)> = Vec::new();
+    let mut multiline_formats: Vec<(String, String)> = Vec::new();
     if format == "markdown" {
         if args.description.is_some() {
-            multiline_formats.push(("System.Description", "Markdown"));
+            multiline_formats.push(("System.Description".to_string(), "Markdown".to_string()));
         }
         if args.acceptance_criteria.is_some() {
-            multiline_formats.push(("Microsoft.VSTS.Common.AcceptanceCriteria", "Markdown"));
+            multiline_formats.push(("Microsoft.VSTS.Common.AcceptanceCriteria".to_string(), "Markdown".to_string()));
         }
         if args.repro_steps.is_some() {
-            multiline_formats.push(("Microsoft.VSTS.TCM.ReproSteps", "Markdown"));
+            multiline_formats.push(("Microsoft.VSTS.TCM.ReproSteps".to_string(), "Markdown".to_string()));
         }
     }
@@ fields_vec
-    let fields_vec: Vec<(&str, serde_json::Value)> = field_map
-        .iter()
-        .map(|(k, v)| (k.as_str(), v.clone()))
-        .collect();
-    let work_item = work_items::create_work_item(
-        client,
+    let fields_vec: Vec<(String, serde_json::Value)> = field_map.into_iter().collect();
+    let work_item = client.create_work_item(
         &args.organization,
         &args.project,
         &args.work_item_type,
         &fields_vec,
         &multiline_formats,
     )
@@ link
-        work_items::link_work_items(
-            client,
+        client.link_work_items(
             &args.organization,
```

#### All tool files and their API call transformations

| File | Old API calls → New trait calls |
|------|------|
| `organizations/list_organizations.rs` | `organizations::get_profile(client)` → `client.get_profile()`, `organizations::list_organizations(client, id)` → `client.list_organizations(id)` |
| `organizations/get_current_user.rs` | `organizations::get_profile(client)` → `client.get_profile()` |
| `projects/list_projects.rs` | `projects::list_projects(client, org)` → `client.list_projects(org)` |
| `teams/list_teams.rs` | `boards::list_teams(client, org, proj)` → `client.list_teams(org, proj)` |
| `teams/get_team.rs` | `boards::get_team(client, org, proj, id)` → `client.get_team(org, proj, id)` |
| `teams/list_team_members.rs` | `client.list_team_members(org, proj, id)` → `client.list_team_members(org, proj, id)` (no change in call, only signature type) |
| `teams/get_team_current_iteration.rs` | `iterations::get_team_current_iteration(client, org, proj, id)` → `client.get_team_current_iteration(org, proj, id)` |
| `teams/boards/list_team_boards.rs` | `boards::list_boards(client, org, proj, id)` → `client.list_boards(org, proj, id)` |
| `teams/boards/get_team_board.rs` | `boards::get_board(client, org, proj, team, board)` → `client.get_board(org, proj, team, board)` |
| `teams/boards/list_board_columns.rs` | `boards::list_board_columns(client, org, proj, team, board)` → `client.list_board_columns(org, proj, team, board)` |
| `teams/boards/list_board_rows.rs` | `boards::list_board_rows(client, org, proj, team, board)` → `client.list_board_rows(org, proj, team, board)` |
| `tags/list_tags.rs` | `tags::list_tags(client, org, proj)` → `client.list_tags(org, proj)` |
| `work_item_types/list_work_item_types.rs` | `boards::list_work_item_types(client, org, proj)` → `client.list_work_item_types(org, proj)` |
| `classification_nodes/list_area_paths.rs` | `classification_nodes::list_area_paths(client, org, proj, parent, depth)` → `client.list_area_paths(org, proj, parent, depth)` |
| `classification_nodes/list_iteration_paths.rs` | `classification_nodes::list_iteration_paths(client, org, proj, parent, depth)` → `client.list_iteration_paths(org, proj, parent, depth)` |
| `work_items/get_work_item.rs` | `work_items::get_work_item(client, org, proj, id, comments)` → `client.get_work_item(org, proj, id, comments)` |
| `work_items/get_work_items.rs` | `work_items::get_work_items(client, org, proj, ids, comments)` → `client.get_work_items(org, proj, ids, comments)` |
| `work_items/create_work_item.rs` | See representative example above (owned types + `client.create_work_item(...)` + `client.link_work_items(...)`). Also update existing unit tests: change local `Vec<(&str, &str)>` to `Vec<(String, String)>` and string literals to `.to_string()` |
| `work_items/update_work_item.rs` | Same owned-type changes as create. `work_items::update_work_item(client, ...)` → `client.update_work_item(...)`. Also update existing unit tests as above |
| `work_items/query_work_items.rs` | `work_items::query_work_items(client, org, proj, query, comments)` → `client.query_work_items(org, proj, query, comments)` |
| `work_items/query_work_items_by_wiql.rs` | `work_items::query_work_items(client, org, proj, query, comments)` → `client.query_work_items(org, proj, query, comments)` |
| `work_items/link_work_items.rs` | `work_items::link_work_items(client, org, proj, src, tgt, type)` → `client.link_work_items(org, proj, src, tgt, type)` |
| `work_items/add_comment.rs` | `work_items::add_comment(client, org, proj, id, text, fmt)` → `client.add_comment(org, proj, id, text, fmt)` |
| `work_items/update_comment.rs` | `work_items::update_comment(client, org, proj, id, cid, text, fmt)` → `client.update_comment(org, proj, id, cid, text, fmt)` |

All tool files MUST keep `CallToolResult` in the `rmcp::model` import — it is required for the function return type `Result<CallToolResult, McpError>`.

**Note on `Option<&str>` parameters**: The `list_area_paths` and `list_iteration_paths` trait methods take `Option<&str>`. When writing mock expectations for these in tests, use `withf()` closure-based matchers instead of `with(eq(...))` to avoid potential lifetime mismatch issues with mockall.

### DoD
- [ ] All 24 tools compile with trait-based client
- [ ] Generated `generated_tools.rs` compiles with `&*self.client`
- [ ] `AzureMcpServer` compiles with `Arc<dyn AzureDevOpsApi + Send + Sync>`

---

## US3: Integration tests

Proves every tool prepends the anti-prompt-injection warning by exercising real tool functions against mocked API.

- [ ] Every MCP tool has an integration test verifying anti-prompt-injection warning prefix
- [ ] Tests use `MockAzureDevOpsApi` from `#[automock]`
- [ ] Tests exercise full tool function → `CallToolResult` path

### Task 3.1: Create `tests/common/mod.rs` (create)

```rust
use mcp_for_azure_devops_boards::mcp::tools::support::UNTRUSTED_CONTENT_WARNING;
use rmcp::model::CallToolResult;

pub fn assert_tool_output_has_warning(result: &CallToolResult) {
    assert!(!result.content.is_empty(), "Tool output must have content");
    let text = extract_text_from_result(result);
    assert!(
        text.starts_with(UNTRUSTED_CONTENT_WARNING),
        "Tool output must start with anti-prompt-injection warning.\nGot: {}",
        &text[..std::cmp::min(text.len(), 120)]
    );
}

pub fn extract_text_from_result(result: &CallToolResult) -> String {
    let content = &result.content[0];
    let json = serde_json::to_value(content).expect("Content should serialize to JSON");
    json.get("text")
        .and_then(|v| v.as_str())
        .expect("Content should have a 'text' field")
        .to_string()
}
```

Note: the `extract_text_from_result` implementation depends on rmcp's `Content` serialization format. Verify during implementation — if `Content` exposes a direct accessor (e.g., `as_text()` or public field), prefer that over JSON round-trip.

### Task 3.2: Integration test files

**File**: `tests/test_tools_organizations.rs`

**Setup**: `mod common; use common::*;` + import `MockAzureDevOpsApi` via `#[cfg(feature = "test-support")] use mcp_for_azure_devops_boards::azure::api_trait::MockAzureDevOpsApi;` + tool arg/fn imports

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_organizations_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_profile` returns `Profile`, `expect_list_organizations` returns `Vec<Organization>` |
| `test_get_current_user_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_profile` returns `Profile` |

**File**: `tests/test_tools_projects.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_projects_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_projects` returns `Vec<Project>` |

**File**: `tests/test_tools_teams.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_teams_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_teams` returns `Vec<Team>` |
| `test_get_team_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_team` returns `Team` |
| `test_list_team_members_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_team_members` returns `Vec<TeamMember>` |
| `test_get_team_current_iteration_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_team_current_iteration` returns `Some(TeamSettingsIteration)` |

**File**: `tests/test_tools_boards.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_team_boards_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_boards` returns `Vec<BoardSummary>` |
| `test_get_team_board_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_board` returns `BoardDetail` |
| `test_list_board_columns_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_board_columns` returns `Vec<BoardColumn>` |
| `test_list_board_rows_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_board_rows` returns `Vec<BoardRow>` |

**File**: `tests/test_tools_tags.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_tags_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_tags` returns `Vec<TagDefinition>` |

**File**: `tests/test_tools_work_item_types.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_work_item_types_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_work_item_types` returns `Vec<WorkItemType>` |

**File**: `tests/test_tools_classification_nodes.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_list_area_paths_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_area_paths` returns `ClassificationNode` |
| `test_list_iteration_paths_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_list_iteration_paths` returns `ClassificationNode` |

**File**: `tests/test_tools_work_items.rs`

| Test | Verifies | Mock setup |
|------|----------|------------|
| `test_get_work_item_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_work_item` returns `Some(WorkItem)` |
| `test_get_work_items_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_get_work_items` returns `Vec<WorkItem>` |
| `test_create_work_item_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_create_work_item` returns `WorkItem` |
| `test_update_work_item_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_update_work_item` returns `WorkItem` |
| `test_query_work_items_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_query_work_items` returns `Vec<WorkItem>` |
| `test_query_work_items_by_wiql_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_query_work_items` returns `Vec<WorkItem>` |
| `test_link_work_items_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_link_work_items` returns `Value` |
| `test_add_comment_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_add_comment` returns `Value` |
| `test_update_comment_has_warning` | Output starts with `UNTRUSTED_CONTENT_WARNING` | `expect_update_comment` returns `Value` |
| `test_get_work_item_api_error_propagates` | API error through trait boundary returns `McpError` | `expect_get_work_item` returns `Err(AzureError::ApiError(...))` |

### DoD
- [ ] All 24 tools covered by at least one warning-prefix test
- [ ] Error propagation through trait boundary verified
- [ ] All integration tests pass

---

## US4: Documentation updates

Ensure project docs and reviewer agents are aware of the new `AzureDevOpsApi` trait convention.

- [ ] `CLAUDE.md` updated with trait convention
- [ ] `.claude/agents/code-reviewer.md` updated
- [ ] `.claude/agents/plan-reviewer.md` updated
- [ ] `docs/PROJECT.md` updated
- [ ] `docs/ARCHITECTURE.md` updated

### Task 4.1: Update `CLAUDE.md` (modify)

Add to section 5 "Architecture Rules", after "Trait-based design and testability":

```diff
+### AzureDevOpsApi trait
+- All MCP tool functions accept `&(dyn AzureDevOpsApi + Send + Sync)` instead of `&AzureDevOpsClient`.
+- `AzureDevOpsApi` is defined in `src/azure/api_trait.rs` with `#[cfg_attr(feature = "test-support", mockall::automock)]`.
+- `AzureDevOpsClient` implements the trait by delegating to the standalone API functions.
+- Integration tests use `MockAzureDevOpsApi` (enabled via `test-support` feature) to verify tool behavior without external services.
```

### Task 4.2: Update `.claude/agents/code-reviewer.md` (modify)

Add to "Architecture Compliance" section after the "MCP tool pattern" bullet:

```diff
+- **AzureDevOpsApi trait**: Tool functions MUST accept `&(dyn AzureDevOpsApi + Send + Sync)`, not `&AzureDevOpsClient`. API calls MUST go through trait methods, not standalone functions. Flag deviations.
```

### Task 4.3: Update `.claude/agents/plan-reviewer.md` (modify)

Add to "Architecture Compliance" section after the "MCP tool pattern" bullet:

```diff
+- **AzureDevOpsApi trait**: Planned tool functions MUST accept `&(dyn AzureDevOpsApi + Send + Sync)`, not `&AzureDevOpsClient`. API calls MUST go through trait methods. Flag deviations.
```

### Task 4.4: Update `docs/PROJECT.md` (modify)

Update "MCP Tool Pattern" section and "Build, Lint, and Test Commands" table to reflect `--features test-support`:

```diff
-3. Function signature: `pub async fn tool_name(client: &AzureDevOpsClient, args: ArgsType) -> Result<CallToolResult, McpError>`.
+3. Function signature: `pub async fn tool_name(client: &(dyn AzureDevOpsApi + Send + Sync), args: ArgsType) -> Result<CallToolResult, McpError>`.
```

### Task 4.5: Update `docs/ARCHITECTURE.md` (modify)

In "Key Data Types" diagram, update `AzureMcpServer`:

```diff
     class AzureMcpServer {
-        -Arc~AzureDevOpsClient~ client
+        -Arc~dyn AzureDevOpsApi~ client
```

### DoD
- [ ] All doc updates concise and accurate

---

## Quality gates

- [ ] `make all` passes (`fmt` → `lint` → `test` → `build`)
