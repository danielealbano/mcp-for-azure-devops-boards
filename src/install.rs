use std::fmt;
use std::path::{Path, PathBuf};

const SERVER_NAME: &str = "mcp-for-azure-devops-boards";

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("Could not determine home directory")]
    HomeDirectoryNotFound,

    #[error("Could not determine config directory")]
    ConfigDirectoryNotFound,

    #[error("Could not determine current directory: {source}")]
    CurrentDirectoryNotFound { source: std::io::Error },

    #[error("Could not detect binary path: {source}")]
    BinaryPathDetection { source: std::io::Error },

    #[error("Failed to create directory {}: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to read config file {}: {source}", path.display())]
    ReadConfig {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to write config file {}: {source}", path.display())]
    WriteConfig {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse JSON config {}: {source}", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("Failed to parse TOML config {}: {source}", path.display())]
    ParseToml {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("Failed to serialize TOML config: {source}")]
    SerializeToml { source: toml::ser::Error },

    #[error("Invalid config format in {}: {detail}", path.display())]
    InvalidConfigFormat { path: PathBuf, detail: String },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum InstallTarget {
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    Vscode,
    Codex,
    GeminiCli,
}

impl fmt::Display for InstallTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallTarget::ClaudeCode => write!(f, "Claude Code"),
            InstallTarget::ClaudeDesktop => write!(f, "Claude Desktop"),
            InstallTarget::Cursor => write!(f, "Cursor"),
            InstallTarget::Vscode => write!(f, "VS Code"),
            InstallTarget::Codex => write!(f, "Codex CLI"),
            InstallTarget::GeminiCli => write!(f, "gemini-cli"),
        }
    }
}

pub fn resolve_config_path(target: &InstallTarget) -> Result<PathBuf, InstallError> {
    match target {
        InstallTarget::ClaudeCode => {
            let home = dirs::home_dir().ok_or(InstallError::HomeDirectoryNotFound)?;
            Ok(home.join(".claude.json"))
        }
        InstallTarget::Cursor => {
            let home = dirs::home_dir().ok_or(InstallError::HomeDirectoryNotFound)?;
            Ok(home.join(".cursor").join("mcp.json"))
        }
        InstallTarget::GeminiCli => {
            let home = dirs::home_dir().ok_or(InstallError::HomeDirectoryNotFound)?;
            Ok(home.join(".gemini").join("settings.json"))
        }
        InstallTarget::Codex => {
            let home = dirs::home_dir().ok_or(InstallError::HomeDirectoryNotFound)?;
            Ok(home.join(".codex").join("config.toml"))
        }
        InstallTarget::Vscode => {
            let cwd = std::env::current_dir()
                .map_err(|e| InstallError::CurrentDirectoryNotFound { source: e })?;
            Ok(cwd.join(".vscode").join("mcp.json"))
        }
        InstallTarget::ClaudeDesktop => {
            let config = dirs::config_dir().ok_or(InstallError::ConfigDirectoryNotFound)?;
            Ok(config.join("Claude").join("claude_desktop_config.json"))
        }
    }
}

pub fn install(
    target: &InstallTarget,
    config_path: &Path,
    binary_path: &Path,
) -> Result<String, InstallError> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| InstallError::CreateDirectory {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    match target {
        InstallTarget::Codex => install_toml(config_path, binary_path)?,
        InstallTarget::Vscode => install_json(config_path, binary_path, "servers", true)?,
        _ => install_json(config_path, binary_path, "mcpServers", false)?,
    }

    Ok(format!(
        "Installed {} configuration at {}",
        target,
        config_path.display()
    ))
}

fn install_json(
    config_path: &Path,
    binary_path: &Path,
    servers_key: &str,
    include_type_stdio: bool,
) -> Result<(), InstallError> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(s) if s.is_empty() => "{}".to_string(),
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}".to_string(),
        Err(e) => {
            return Err(InstallError::ReadConfig {
                path: config_path.to_path_buf(),
                source: e,
            });
        }
    };

    let mut root: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| InstallError::ParseJson {
            path: config_path.to_path_buf(),
            source: e,
        })?;

    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| InstallError::InvalidConfigFormat {
            path: config_path.to_path_buf(),
            detail: "expected JSON object at root".to_string(),
        })?;

    if !root_obj.contains_key(servers_key) {
        root_obj.insert(
            servers_key.to_string(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
    }

    let servers = root_obj
        .get_mut(servers_key)
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| InstallError::InvalidConfigFormat {
            path: config_path.to_path_buf(),
            detail: format!("expected \"{servers_key}\" to be an object"),
        })?;

    let binary_str = binary_path.to_string_lossy();
    let mut entry = serde_json::Map::new();
    if include_type_stdio {
        entry.insert(
            "type".to_string(),
            serde_json::Value::String("stdio".to_string()),
        );
    }
    entry.insert(
        "command".to_string(),
        serde_json::Value::String(binary_str.into_owned()),
    );

    servers.insert(SERVER_NAME.to_string(), serde_json::Value::Object(entry));

    let output = serde_json::to_string_pretty(&root).map_err(|e| InstallError::ParseJson {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    std::fs::write(config_path, format!("{output}\n")).map_err(|e| InstallError::WriteConfig {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

fn install_toml(config_path: &Path, binary_path: &Path) -> Result<(), InstallError> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(s) if s.is_empty() => String::new(),
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(InstallError::ReadConfig {
                path: config_path.to_path_buf(),
                source: e,
            });
        }
    };

    let mut root: toml::Value = toml::from_str(&content).map_err(|e| InstallError::ParseToml {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    let root_table = root
        .as_table_mut()
        .ok_or_else(|| InstallError::InvalidConfigFormat {
            path: config_path.to_path_buf(),
            detail: "expected TOML table at root".to_string(),
        })?;

    if !root_table.contains_key("mcp_servers") {
        root_table.insert(
            "mcp_servers".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }

    let mcp_servers = root_table
        .get_mut("mcp_servers")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| InstallError::InvalidConfigFormat {
            path: config_path.to_path_buf(),
            detail: "expected \"mcp_servers\" to be a table".to_string(),
        })?;

    let binary_str = binary_path.to_string_lossy().into_owned();
    let mut entry = toml::map::Map::new();
    entry.insert("command".to_string(), toml::Value::String(binary_str));

    mcp_servers.insert(SERVER_NAME.to_string(), toml::Value::Table(entry));

    let output =
        toml::to_string_pretty(&root).map_err(|e| InstallError::SerializeToml { source: e })?;

    std::fs::write(config_path, output).map_err(|e| InstallError::WriteConfig {
        path: config_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const TEST_BINARY_PATH: &str = "/test/path/mcp-for-azure-devops-boards";

    fn binary_path() -> PathBuf {
        PathBuf::from(TEST_BINARY_PATH)
    }

    #[test]
    fn test_install_claude_code_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".claude.json");

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
        assert!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]
                .get("type")
                .is_none()
        );
    }

    #[test]
    fn test_install_claude_desktop_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("Claude").join("claude_desktop_config.json");

        install(&InstallTarget::ClaudeDesktop, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_cursor_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".cursor").join("mcp.json");

        install(&InstallTarget::Cursor, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_gemini_cli_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".gemini").join("settings.json");

        install(&InstallTarget::GeminiCli, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_vscode_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".vscode").join("mcp.json");

        install(&InstallTarget::Vscode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["servers"]["mcp-for-azure-devops-boards"]["type"],
            "stdio"
        );
        assert_eq!(
            content["servers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_codex_creates_config() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join(".codex").join("config.toml");

        install(&InstallTarget::Codex, &config_path, &binary_path()).unwrap();

        let content: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcp_servers"]["mcp-for-azure-devops-boards"]["command"]
                .as_str()
                .unwrap(),
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_json_preserves_existing_keys() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(&config_path, r#"{"theme": "dark", "fontSize": 14}"#).unwrap();

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(content["theme"], "dark");
        assert_eq!(content["fontSize"], 14);
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_json_preserves_other_servers() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"mcpServers": {"other-server": {"command": "/other/bin"}}}"#,
        )
        .unwrap();

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["other-server"]["command"],
            "/other/bin"
        );
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_json_updates_existing_entry() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"mcpServers": {"mcp-for-azure-devops-boards": {"command": "/old/path"}}}"#,
        )
        .unwrap();

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_toml_preserves_existing_keys() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        std::fs::write(&config_path, "model = \"gpt-4\"\n").unwrap();

        install(&InstallTarget::Codex, &config_path, &binary_path()).unwrap();

        let content: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(content["model"].as_str().unwrap(), "gpt-4");
        assert_eq!(
            content["mcp_servers"]["mcp-for-azure-devops-boards"]["command"]
                .as_str()
                .unwrap(),
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_toml_preserves_other_servers() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        std::fs::write(
            &config_path,
            "[mcp_servers.other-server]\ncommand = \"/other/bin\"\n",
        )
        .unwrap();

        install(&InstallTarget::Codex, &config_path, &binary_path()).unwrap();

        let content: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcp_servers"]["other-server"]["command"]
                .as_str()
                .unwrap(),
            "/other/bin"
        );
        assert_eq!(
            content["mcp_servers"]["mcp-for-azure-devops-boards"]["command"]
                .as_str()
                .unwrap(),
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_toml_updates_existing_entry() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        std::fs::write(
            &config_path,
            "[mcp_servers.mcp-for-azure-devops-boards]\ncommand = \"/old/path\"\n",
        )
        .unwrap();

        install(&InstallTarget::Codex, &config_path, &binary_path()).unwrap();

        let content: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcp_servers"]["mcp-for-azure-devops-boards"]["command"]
                .as_str()
                .unwrap(),
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("deep").join("nested").join("config.json");

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        assert!(config_path.exists());
        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_returns_success_message() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");

        let msg = install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        assert!(
            msg.contains("Claude Code"),
            "message should contain target name: {msg}"
        );
        assert!(
            msg.contains(&config_path.display().to_string()),
            "message should contain config path: {msg}"
        );
    }

    #[test]
    fn test_install_json_invalid_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(&config_path, "{broken").unwrap();

        let result = install(&InstallTarget::ClaudeCode, &config_path, &binary_path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, InstallError::ParseJson { .. }),
            "expected ParseJson, got: {err}"
        );
    }

    #[test]
    fn test_install_toml_invalid_toml_returns_error() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        std::fs::write(&config_path, "[broken\ninvalid toml").unwrap();

        let result = install(&InstallTarget::Codex, &config_path, &binary_path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, InstallError::ParseToml { .. }),
            "expected ParseToml, got: {err}"
        );
    }

    #[test]
    fn test_install_json_empty_file_treated_as_new() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(&config_path, "").unwrap();

        install(&InstallTarget::ClaudeCode, &config_path, &binary_path()).unwrap();

        let content: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["mcp-for-azure-devops-boards"]["command"],
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_install_json_root_not_object_returns_error() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.json");
        std::fs::write(&config_path, "[1, 2, 3]").unwrap();

        let result = install(&InstallTarget::ClaudeCode, &config_path, &binary_path());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, InstallError::InvalidConfigFormat { .. }),
            "expected InvalidConfigFormat, got: {err}"
        );
    }

    #[test]
    fn test_install_toml_empty_file_treated_as_new() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.toml");
        std::fs::write(&config_path, "").unwrap();

        install(&InstallTarget::Codex, &config_path, &binary_path()).unwrap();

        let content: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            content["mcp_servers"]["mcp-for-azure-devops-boards"]["command"]
                .as_str()
                .unwrap(),
            TEST_BINARY_PATH
        );
    }

    #[test]
    fn test_resolve_config_path_claude_code() {
        let path = resolve_config_path(&InstallTarget::ClaudeCode).unwrap();
        assert!(
            path.ends_with(".claude.json"),
            "expected path ending with .claude.json, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_config_path_cursor() {
        let path = resolve_config_path(&InstallTarget::Cursor).unwrap();
        assert!(
            path.ends_with(".cursor/mcp.json") || path.ends_with(".cursor\\mcp.json"),
            "expected path ending with .cursor/mcp.json, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_config_path_gemini_cli() {
        let path = resolve_config_path(&InstallTarget::GeminiCli).unwrap();
        assert!(
            path.ends_with(".gemini/settings.json") || path.ends_with(".gemini\\settings.json"),
            "expected path ending with .gemini/settings.json, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_config_path_codex() {
        let path = resolve_config_path(&InstallTarget::Codex).unwrap();
        assert!(
            path.ends_with(".codex/config.toml") || path.ends_with(".codex\\config.toml"),
            "expected path ending with .codex/config.toml, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_config_path_vscode() {
        let path = resolve_config_path(&InstallTarget::Vscode).unwrap();
        assert!(
            path.ends_with(".vscode/mcp.json") || path.ends_with(".vscode\\mcp.json"),
            "expected path ending with .vscode/mcp.json, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_resolve_config_path_claude_desktop() {
        let path = resolve_config_path(&InstallTarget::ClaudeDesktop).unwrap();
        assert!(
            path.ends_with("Claude/claude_desktop_config.json")
                || path.ends_with("Claude\\claude_desktop_config.json"),
            "expected path ending with Claude/claude_desktop_config.json, got: {}",
            path.display()
        );
    }
}
