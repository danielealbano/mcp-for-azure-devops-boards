# MCP for Azure DevOps Boards

[![CI - PR - Build & Test](https://github.com/danielealbano/mcp-for-azure-devops-boards/actions/workflows/ci-pr-build-and-test.yml/badge.svg)](https://github.com/danielealbano/mcp-for-azure-devops-boards/actions/workflows/ci-pr-build-and-test.yml)
[![CD - Tag - Build & Release](https://github.com/danielealbano/mcp-for-azure-devops-boards/actions/workflows/cd-tag-build-and-release.yml/badge.svg)](https://github.com/danielealbano/mcp-for-azure-devops-boards/actions/workflows/cd-tag-build-and-release.yml)

A Model Context Protocol (MCP) server for interacting with Azure DevOps Boards and Work Items, written in Rust.

## Features

-   **Work Item Management**: Create, update, get, and query work items.
-   **Board Integration**: List teams, boards, and fetch board items.

-   **WIQL Support**: Run custom WIQL queries.
-   **Simplified Output**: Optimized JSON output for LLM consumption (reduced token usage).

## Installation

### macOS (Homebrew)

```bash
brew tap danielealbano/mcp-tools
brew install mcp-for-azure-devops-boards
```

The path to the binary will be `/opt/homebrew/bin/mcp-for-azure-devops-boards`.

Check out the section [MCP Configuration](#mcp-configuration) for how to configure your preferred AI (MCP) client.

### Configuration

| Setting | Description | CLI Flag | Env Variable |
| :--- | :--- | :--- | :--- |
| **Server Mode** | Run as HTTP server instead of stdio | `--server` | N/A |
| **Port** | Port for HTTP server (default: 3000) | `--port` | N/A |

*Note: If `--server` is not specified, the software will run in stdio mode.*

### Authentication

This server leverages standard Azure authentication mechanisms (like `az` or `azd`) to query Azure DevOps.

#### Installing Azure CLI

If you don't have the Azure CLI installed:

**macOS (Homebrew):**
```bash
brew install azure-cli
```

**Windows (Chocolatey):**
```powershell
choco install azure-cli
```

For other installation methods, see the [official Azure CLI installation guide](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli).

#### Logging In

To authenticate, run:
```bash
az login
```

## Usage

### Stdio Mode (Default)

This is the standard mode for MCP clients (like Claude Desktop or Cursor). **This mode is preferred for security as it ensures no credentials are shared over the network.**

```bash
path/to/mcp-for-azure-devops-boards
```

### HTTP Server Mode

You can also run it as an HTTP server (SSE). **Note that in this mode, the server listens on `0.0.0.0` (all interfaces).**

```bash
path/to/mcp-for-azure-devops-boards --server --port 3000
```

### MCP Configuration

*Note: Make sure you have run `az login` in your terminal so the process can pick up the credentials.*

#### Claude Desktop

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "azure-devops-boards": {
      "command": "path/to/mcp-for-azure-devops-boards"
    }
  }
}
```

### Available Tools

> *This software is currently in development. The tools and their parameters are subject to change.*

The server exposes the following tools for MCP clients:

#### Discovery

-   **`azdo_list_organizations`**: List all Azure DevOps organizations the authenticated user has access to.
    -   **Required**: None (uses authenticated user's credentials)
-   **`azdo_list_projects`**: List all projects in an Azure DevOps organization.
    -   **Required**: `organization`

#### Work Items

> **Note**: All work item tools require `organization` and `project` parameters.

-   **`azdo_create_work_item`**: Create a new work item.
    -   **Required**: `organization`, `project`, `work_item_type`, `title`
    -   **Optional**: `description`, `assigned_to`, `area_path`, `iteration`, `state`, `board_column`, `board_row`, `priority`, `severity`, `story_points`, `effort`, `remaining_work`, `tags`, `activity`, `parent_id`, `start_date`, `target_date`, `acceptance_criteria`, `repro_steps`, `fields` (JSON string for custom fields).
-   **`azdo_update_work_item`**: Update an existing work item.
    -   **Required**: `organization`, `project`, `id`
    -   **Optional**: All fields available in creation.
-   **`azdo_get_work_item`**: Get details of a specific work item.
    -   **Required**: `organization`, `project`, `id`
    -   **Optional**: `include_latest_n_comments` (number of recent comments to include, -1 for all)
-   **`azdo_get_work_items`**: Get multiple work items by their IDs.
    -   **Required**: `organization`, `project`, `ids` (array of work item IDs)
    -   **Optional**: `include_latest_n_comments` (number of recent comments to include, -1 for all)
-   **`azdo_query_work_items`**: Query work items using structured filters.
    -   **Required**: `organization`, `project`
    -   **Optional Filters**: `area_path`, `iteration`, `created_date_from/to`, `modified_date_from/to`.
    -   **Inclusion Lists**: `include_board_column`, `include_board_row`, `include_work_item_type`, `include_state`, `include_assigned_to`, `include_tags`.
    -   **Exclusion Lists**: `exclude_board_column`, `exclude_board_row`, `exclude_work_item_type`, `exclude_state`, `exclude_assigned_to`, `exclude_tags`.
    -   **Optional**: `include_latest_n_comments` (number of recent comments to include, -1 for all)
-   **`azdo_query_work_items_wiql`**: Execute a raw WIQL (Work Item Query Language) query.
    -   **Required**: `organization`, `project`, `query`
    -   **Optional**: `include_latest_n_comments` (number of recent comments to include, -1 for all)
-   **`azdo_add_comment`**: Add a comment to a work item.
    -   **Required**: `organization`, `project`, `work_item_id`, `text`
-   **`azdo_link_work_items`**: Create a relationship between two work items.
    -   **Required**: `organization`, `project`, `source_id`, `target_id`, `link_type` (Parent, Child, Related, Duplicate, Dependency).

#### Boards & Teams

> **Note**: All board and team tools require `organization` and `project` parameters.

-   **`azdo_list_teams`**: List all teams in the project.
    -   **Required**: `organization`, `project`
-   **`azdo_get_team`**: Get details of a specific team.
    -   **Required**: `organization`, `project`, `team_id`
-   **`azdo_list_boards`**: List boards for a specific team.
    -   **Required**: `organization`, `project`, `team_id`
-   **`azdo_get_board`**: Get details of a specific board.
    -   **Required**: `organization`, `project`, `team_id`, `board_id`
-   **`azdo_list_work_item_types`**: List all available work item types in the project.
    -   **Required**: `organization`, `project`
-   **`azdo_list_tags`**: List all tags in use in the project.
    -   **Required**: `organization`, `project`
-   **`azdo_team_get_current_iteration`**: Get the current active iteration/sprint for a team.
    -   **Required**: `organization`, `project`, `team_id`
-   **`azdo_team_get_iterations`**: Get all iterations/sprints for a team.
    -   **Required**: `organization`, `project`, `team_id`



## Contributing

We welcome contributions!

1.  **Fork** the repository.
2.  Create a new **branch** for your feature or bugfix (`git checkout -b feature/amazing-feature`).
3.  **Commit** your changes.
4.  **Push** to your branch.
5.  Open a **Pull Request**.

### Building from Source

#### Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
-   [Azure CLI](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli) (required for local authentication)

#### Steps

1.  Clone the repository:
    ```bash
    git clone https://github.com/danielealbano/mcp-for-azure-devops-boards.git
    cd mcp-for-azure-devops-boards
    ```

2.  Build the project:
    ```bash
    cargo build --release
    ```

### Tooling

- Run tests: `cargo test`
- Check code style: `cargo fmt --check`
- Linting: `cargo clippy`

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
