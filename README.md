# Azure DevOps MCP Server

A Model Context Protocol (MCP) server for interacting with Azure DevOps Boards and Work Items, written in Rust.

## Features

-   **Work Item Management**: Create, update, get, and query work items.
-   **Board Integration**: List teams, boards, and fetch board items.
-   **Attachments**: Upload and download attachments.
-   **WIQL Support**: Run custom WIQL queries.
-   **Simplified Output**: Optimized JSON output for LLM consumption (reduced token usage).

## Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
-   [Azure CLI](https://learn.microsoft.com/en-us/cli/azure/install-azure-cli) (required for local authentication)

## Installation

1.  Clone the repository:
    ```bash
    git clone https://github.com/yourusername/azure-devops-mcp.git
    cd azure-devops-mcp
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

The executable will be located at `./target/release/azure-devops-mcp`.

### Stdio Mode (Default)

This is the standard mode for MCP clients (like Claude Desktop or Cursor).

```bash
./target/release/azure-devops-mcp --organization <YOUR_ORG> --project <YOUR_PROJECT>
```

Or using environment variables:
```bash
export AZDO_ORGANIZATION=myorg
export AZDO_PROJECT=myproject
./target/release/azure-devops-mcp
```

### HTTP Server Mode

You can also run it as an HTTP server (SSE).

```bash
./target/release/azure-devops-mcp --server --port 3000 --organization <YOUR_ORG> --project <YOUR_PROJECT>
```

## MCP Client Configuration

### Claude Desktop

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "azure-devops-boards": {
      "command": "/absolute/path/to/azure-devops-mcp/target/release/azure-devops-mcp",
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
