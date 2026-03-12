#![cfg(feature = "e2e-tests")]

use std::path::PathBuf;

use base64::Engine;
use mcp_for_azure_devops_boards::install::{InstallTarget, install};
use testcontainers::core::{ExecCommand, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

const SERVER_NAME: &str = "mcp-for-azure-devops-boards";

async fn run_e2e_install_test(
    image_name: &str,
    target: InstallTarget,
    config_path_in_container: &str,
    verify_command: Vec<String>,
    env_vars: Vec<(&str, &str)>,
) {
    let tmp = tempfile::TempDir::new().unwrap();
    let config_path = tmp.path().join("config_file");
    let binary_path = PathBuf::from("/usr/local/bin/mcp-for-azure-devops-boards");

    install(&target, &config_path, &binary_path).unwrap();

    let config_content = std::fs::read_to_string(&config_path).unwrap();
    let encoded = base64::engine::general_purpose::STANDARD.encode(config_content.as_bytes());

    let parent_dir = std::path::Path::new(config_path_in_container)
        .parent()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let setup_script = format!(
        "mkdir -p '{parent_dir}' && echo '{encoded}' | base64 -d > '{config_path_in_container}'"
    );

    let mut image = GenericImage::new(image_name, "latest")
        .with_wait_for(WaitFor::seconds(2))
        .with_cmd(vec!["sleep", "infinity"]);
    for (key, val) in &env_vars {
        image = image.with_env_var(*key, *val);
    }
    let container = image
        .start()
        .await
        .unwrap_or_else(|e| panic!("Failed to start container {image_name}: {e}"));

    container
        .exec(ExecCommand::new(vec!["bash", "-c", &setup_script]))
        .await
        .unwrap_or_else(|e| panic!("Failed to write config into container: {e}"));

    let mut result = container
        .exec(ExecCommand::new(verify_command.clone()))
        .await
        .unwrap_or_else(|e| panic!("Failed to exec verify command: {e}"));

    let stdout_bytes = result.stdout_to_vec().await.unwrap_or_default();
    let stderr_bytes = result.stderr_to_vec().await.unwrap_or_default();
    let stdout = String::from_utf8_lossy(&stdout_bytes);
    let stderr = String::from_utf8_lossy(&stderr_bytes);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains(SERVER_NAME),
        "Expected output to contain '{SERVER_NAME}', got stdout: {stdout}, stderr: {stderr}"
    );
}

#[tokio::test]
async fn test_e2e_claude_code_recognizes_config() {
    run_e2e_install_test(
        "mcp-test-claude-code",
        InstallTarget::ClaudeCode,
        "/root/.claude.json",
        vec!["claude".to_string(), "mcp".to_string(), "list".to_string()],
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_e2e_cursor_recognizes_config() {
    run_e2e_install_test(
        "mcp-test-cursor",
        InstallTarget::Cursor,
        "/root/.cursor/mcp.json",
        vec!["agent".to_string(), "mcp".to_string(), "list".to_string()],
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_e2e_gemini_cli_recognizes_config() {
    run_e2e_install_test(
        "mcp-test-gemini-cli",
        InstallTarget::GeminiCli,
        "/root/.gemini/settings.json",
        vec!["gemini".to_string(), "mcp".to_string(), "list".to_string()],
        vec![("GOOGLE_GENAI_USE_GCA", "true")],
    )
    .await;
}
