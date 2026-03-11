# Architecture

## System Overview

```mermaid
graph LR
    LLM["LLM Agent<br/>(Cursor, Claude Desktop)"]
    MCP["MCP Server<br/>mcp-for-azure-devops-boards"]
    AzDO["Azure DevOps<br/>REST API"]

    LLM -->|"stdio / HTTP"| MCP
    MCP -->|"Bearer token<br/>REST API v7.1"| AzDO
```

## Project Structure

```
├── Cargo.toml                    # Workspace root + main crate
├── Cargo.lock
├── build.rs                      # Scans #[mcp_tool] → generates tool router
├── Makefile                      # build, test, lint, fmt targets
├── Dockerfile                    # Multi-stage: rust:alpine → alpine
├── tests/                        # Integration tests (anti-prompt-injection, tool behavior)
│   ├── common/
│   │   └── mod.rs                # Shared helpers (assert_tool_output_has_warning)
│   ├── test_tools_organizations.rs
│   ├── test_tools_projects.rs
│   ├── test_tools_teams.rs
│   ├── test_tools_boards.rs
│   ├── test_tools_tags.rs
│   ├── test_tools_work_item_types.rs
│   ├── test_tools_classification_nodes.rs
│   └── test_tools_work_items.rs
├── src/
│   ├── main.rs                   # CLI entry (clap), transport selection
│   ├── lib.rs                    # Library root: re-exports modules
│   ├── compact_llm.rs            # Compact JSON serializer for LLM output
│   ├── azure/                    # Azure DevOps API client layer
│   │   ├── mod.rs
│   │   ├── client.rs             # AzureDevOpsClient, AzureError, auth, HTTP helpers
│   │   ├── api_trait.rs          # AzureDevOpsApi trait + MockAzureDevOpsApi (test-support feature)
│   │   ├── models.rs             # Shared data types (WorkItem, Board, Comment, etc.)
│   │   ├── boards.rs             # Boards API
│   │   ├── classification_nodes.rs # Area/Iteration paths API
│   │   ├── iterations.rs         # Iterations API
│   │   ├── organizations.rs      # Organizations API
│   │   ├── projects.rs           # Projects API
│   │   ├── tags.rs               # Tags API
│   │   ├── teams.rs              # Teams API
│   │   └── work_items.rs         # Work items API (CRUD, WIQL, comments, links)
│   ├── mcp/                      # MCP server layer
│   │   ├── mod.rs
│   │   ├── server.rs             # AzureMcpServer, ServerHandler, includes generated_tools.rs
│   │   └── tools/                # MCP tool implementations
│   │       ├── mod.rs
│   │       ├── classification_nodes/   # list_area_paths, list_iteration_paths
│   │       ├── organizations/          # list_organizations, get_current_user
│   │       ├── projects/               # list_projects
│   │       ├── tags/                   # list_tags
│   │       ├── teams/                  # list_teams, get_team, list_team_members, get_team_current_iteration
│   │       │   └── boards/             # list_team_boards, get_team_board, list_board_columns, list_board_rows
│   │       ├── work_item_types/        # list_work_item_types
│   │       ├── work_items/             # create, update, get, get_many, query, wiql_query, link, add_comment
│   │       └── support/                # Shared utilities (CSV, JSON simplification, deserializers)
│   └── server/                   # HTTP transport
│       ├── mod.rs
│       └── http.rs               # hyper + rmcp StreamableHttpService
├── mcp-tools-codegen/            # Proc-macro crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs                # #[mcp_tool] attribute macro
├── .github/workflows/            # CI/CD
│   ├── ci-pr-build-and-test.yml
│   └── cd-tag-build-and-release.yml
└── docs/
    ├── PROJECT.md
    └── ARCHITECTURE.md
```

## Layer Architecture

```mermaid
graph TD
    subgraph "Entry Point"
        MAIN["main.rs<br/>CLI (clap) + transport selection"]
    end

    subgraph "MCP Layer"
        SERVER["mcp/server.rs<br/>AzureMcpServer + ToolRouter"]
        TOOLS["mcp/tools/*<br/>24 MCP tool functions"]
        SUPPORT["mcp/tools/support/*<br/>CSV, JSON simplification"]
        CODEGEN["build.rs + mcp-tools-codegen<br/>Tool router code generation"]
    end

    subgraph "Transport Layer"
        STDIO["rmcp stdio transport"]
        HTTP["server/http.rs<br/>hyper + StreamableHttpService"]
    end

    subgraph "Azure DevOps Client"
        CLIENT["azure/client.rs<br/>AzureDevOpsClient + AzureError"]
        API["azure/*.rs<br/>API modules (boards, teams, work_items, etc.)"]
        MODELS["azure/models.rs<br/>Data types"]
    end

    subgraph "External"
        AUTH["azure_identity<br/>DefaultAzureCredential"]
        AZDO["Azure DevOps REST API v7.1"]
    end

    MAIN --> SERVER
    MAIN --> STDIO
    MAIN --> HTTP
    SERVER --> TOOLS
    TOOLS --> SUPPORT
    TOOLS --> API
    CODEGEN -.->|generates| SERVER
    API --> CLIENT
    API --> MODELS
    CLIENT --> AUTH
    CLIENT --> AZDO
```

## Request Flow

```mermaid
sequenceDiagram
    participant LLM as LLM Agent
    participant Transport as Transport (stdio/HTTP)
    participant Router as ToolRouter
    participant Tool as MCP Tool Function
    participant API as Azure API Module
    participant Client as AzureDevOpsClient
    participant AzDO as Azure DevOps

    LLM->>Transport: MCP tool call (JSON-RPC)
    Transport->>Router: Route by tool name
    Router->>Tool: Deserialize args, invoke
    Tool->>API: Call API function
    API->>Client: HTTP request builder
    Client->>Client: get_token() via DefaultAzureCredential
    Client->>AzDO: HTTP request (Bearer token)
    AzDO-->>Client: JSON response
    Client-->>API: Deserialized response
    API-->>Tool: Domain types
    Tool->>Tool: Simplify + compact output
    Tool-->>Router: CallToolResult
    Router-->>Transport: MCP response
    Transport-->>LLM: JSON-RPC response
```

## MCP Tools

| Category | Tool | Description |
|---|---|---|
| **Organizations** | `azdo_list_organizations` | List accessible organizations |
| | `azdo_get_current_user` | Get current authenticated user |
| **Projects** | `azdo_list_projects` | List projects in an organization |
| **Tags** | `azdo_list_tags` | List tags in a project |
| **Work Item Types** | `azdo_list_work_item_types` | List work item types in a project |
| **Classification** | `azdo_list_iteration_paths` | List iteration paths |
| | `azdo_list_area_paths` | List area paths |
| **Teams** | `azdo_list_teams` | List teams in a project |
| | `azdo_get_team` | Get team details |
| | `azdo_list_team_members` | List team members |
| | `azdo_get_team_current_iteration` | Get current iteration for a team |
| **Boards** | `azdo_list_team_boards` | List boards for a team |
| | `azdo_get_team_board` | Get board details |
| | `azdo_list_board_columns` | List board columns |
| | `azdo_list_board_rows` | List board rows/swimlanes |
| **Work Items** | `azdo_create_work_item` | Create a work item |
| | `azdo_update_work_item` | Update a work item |
| | `azdo_get_work_item` | Get work item by ID |
| | `azdo_get_work_items` | Get multiple work items by IDs |
| | `azdo_query_work_items` | Query work items (natural language → WIQL) |
| | `azdo_query_work_items_by_wiql` | Query work items by raw WIQL |
| | `azdo_link_work_items` | Link two work items |
| | `azdo_add_comment` | Add comment to a work item |
| | `azdo_update_comment` | Update a comment on a work item |

## Key Data Types

```mermaid
classDiagram
    class AzureDevOpsClient {
        -Client client
        -Arc~DefaultAzureCredential~ credential
        +new() Self
        +get(org, project, path) Result~T~
        +post(org, project, path, body) Result~T~
        +patch(org, project, path, body) Result~T~
        +org_request(org, method, path, body) Result~T~
        +team_request(org, project, method, team, path, body) Result~T~
        +vssps_request(method, path, body) Result~T~
    }

    class AzureError {
        <<enum>>
        AuthError(azure_core::Error)
        HttpError(reqwest::Error)
        SerdeJson(serde_json::Error)
        ApiError(String)
    }

    class AzureMcpServer {
        -Arc~dyn AzureDevOpsApi~ client
        -ToolRouter~Self~ tool_router
        +new(client) Self
    }

    class WorkItem {
        +u32 id
        +HashMap~String, Value~ fields
        +Option~String~ url
        +Option~Vec~Comment~~ comments
    }

    AzureMcpServer --> AzureDevOpsClient : Arc
    AzureDevOpsClient ..> AzureError : returns
    AzureDevOpsClient ..> WorkItem : returns
```

## Shared State Model

```mermaid
graph LR
    subgraph "Per-Connection"
        TOOL["Tool function<br/>(stateless)"]
    end

    subgraph "Shared (Arc)"
        CLIENT["AzureDevOpsClient<br/>reqwest::Client + credential"]
    end

    subgraph "Per-Request"
        TOKEN["Bearer token<br/>(fetched each request)"]
    end

    TOOL -->|"&self.client"| CLIENT
    CLIENT -->|"get_token()"| TOKEN
```

- `AzureMcpServer` is `Clone` (wraps `Arc<AzureDevOpsClient>`)
- Each HTTP connection gets a clone of `AzureMcpServer`
- `AzureDevOpsClient` contains `reqwest::Client` (internally Arc'd, connection-pooled) and `Arc<DefaultAzureCredential>`
- Bearer tokens are fetched per-request (credential SDK handles caching)
