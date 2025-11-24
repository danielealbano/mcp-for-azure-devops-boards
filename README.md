# Azure DevOps MCP Server

A Model Context Protocol (MCP) server for interacting with Azure DevOps Boards and Work Items, written in Rust.

## Features

-   **Work Item Management**: Create, update, get, and query work items.
-   **Board Integration**: List teams, boards, and fetch board items.
-   **Attachments**: Upload and download attachments.
-   **WIQL Support**: Run custom WIQL queries.
-   **Simplified Output**: Optimized JSON output for LLM consumption (reduced token usage).

## Installation

### macOS (Homebrew)

```bash
brew tap danielealbano/mcp-tools
brew install azure-devops-boards-mcp-rust
```

### Building from Source

#### Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
-   [Azure CLI](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli) (required for local authentication)

#### Steps

1.  Clone the repository:
    ```bash
    git clone https://github.com/danielealbano/azure-devops-boards-mcp-rust.git
    cd azure-devops-boards-mcp-rust
    ```

2.  Build the project:
    ```bash
    cargo build --release
    ```

## Configuration

The server requires the following configuration:

### Configuration

| Setting | Description | CLI Flag | Env Variable |
| :--- | :--- | :--- | :--- |
| **Organization** | Azure DevOps organization name | `--organization` | `AZDO_ORGANIZATION` |
| **Project** | Azure DevOps project name | `--project` | `AZDO_PROJECT` |
| **Server Mode** | Run as HTTP server instead of stdio | `--server` | N/A |
| **Port** | Port for HTTP server (default: 3000) | `--port` | N/A |

*Note: If `--server` is not specified, the software will run in stdio mode.*

### Authentication

This server leverages standard Azure authentication mechanisms (like `az` or `azd`) to query Azure DevOps.

To authenticate, if you haven't already, run:
```bash
az login
```

## Usage

First, build the project in release mode:

```bash
cargo build --release
```

The executable will be located at `./target/release/azure-devops-boards-mcp-rust`.

### Stdio Mode (Default)

This is the standard mode for MCP clients (like Claude Desktop or Cursor). **This mode is preferred for security as it ensures no credentials are shared over the network.**

```bash
./target/release/azure-devops-boards-mcp-rust --organization <YOUR_ORG> --project <YOUR_PROJECT>
```

Or using environment variables:
```bash
export AZDO_ORGANIZATION=myorg
export AZDO_PROJECT=myproject
./target/release/azure-devops-boards-mcp-rust
```

### HTTP Server Mode

You can also run it as an HTTP server (SSE). **Note that in this mode, the server listens on `0.0.0.0` (all interfaces).**

```bash
./target/release/azure-devops-boards-mcp-rust --server --port 3000 --organization <YOUR_ORG> --project <YOUR_PROJECT>
```

## MCP Client Configuration

### Claude Desktop

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "azure-devops-boards": {
      "command": "/absolute/path/to/azure-devops-boards-mcp-rust/target/release/azure-devops-boards-mcp-rust",
      "args": [
        "--organization",
        "YOUR_ORG_NAME",
        "--project",
        "YOUR_PROJECT_NAME"
      ]
    }
  }
}
```

*Note: Make sure you have run `az login` in your terminal so the process can pick up the credentials.*

## Available Tools

> *This software is currently in development. The tools and their parameters are subject to change.*

The server exposes the following tools for MCP clients:

### Work Items

-   **`create_work_item`**: Create a new work item.
    -   **Required**: `work_item_type`, `title`
    -   **Optional**: `description`, `assigned_to`, `area_path`, `iteration`, `state`, `board_column`, `board_row`, `priority`, `severity`, `story_points`, `effort`, `remaining_work`, `tags`, `activity`, `parent_id`, `start_date`, `target_date`, `acceptance_criteria`, `repro_steps`, `fields` (JSON string for custom fields).
-   **`update_work_item`**: Update an existing work item.
    -   **Required**: `id`
    -   **Optional**: All fields available in creation.
-   **`get_work_item`**: Get details of a specific work item.
    -   **Required**: `id`
-   **`query_work_items`**: Query work items using structured filters.
    -   **Optional Filters**: `area_path`, `iteration`, `created_date_from/to`, `modified_date_from/to`.
    -   **Inclusion Lists**: `include_board_column`, `include_board_row`, `include_work_item_type`, `include_state`, `include_assigned_to`, `include_tags`.
    -   **Exclusion Lists**: `exclude_board_column`, `exclude_board_row`, `exclude_work_item_type`, `exclude_state`, `exclude_assigned_to`, `exclude_tags`.
-   **`query_work_items_wiql`**: Execute a raw WIQL (Work Item Query Language) query.
    -   **Required**: `query`
-   **`add_comment`**: Add a comment to a work item.
    -   **Required**: `work_item_id`, `text`
-   **`link_work_items`**: Create a relationship between two work items.
    -   **Required**: `source_id`, `target_id`, `link_type` (Parent, Child, Related, Duplicate, Dependency).

### Boards & Teams

-   **`list_teams`**: List all teams in the project.
-   **`get_team`**: Get details of a specific team.
    -   **Required**: `team_id`
-   **`list_boards`**: List boards for a specific team.
    -   **Required**: `team_id`
-   **`get_board`**: Get details of a specific board.
    -   **Required**: `team_id`, `board_id`
-   **`list_work_item_types`**: List all available work item types in the project.

### Attachments

-   **`upload_attachment`**: Upload a file attachment.
    -   **Required**: `file_name`, `content` (Base64 encoded).
-   **`download_attachment`**: Download a file attachment.
    -   **Required**: `id`
    -   **Optional**: `file_name`

## Contributing

We welcome contributions!

1.  **Fork** the repository.
2.  Create a new **branch** for your feature or bugfix (`git checkout -b feature/amazing-feature`).
3.  **Commit** your changes.
4.  **Push** to your branch.
5.  Open a **Pull Request**.

### Development

-   Run tests: `cargo test`
-   Check code style: `cargo fmt --check`
-   Linting: `cargo clippy`

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
