use crate::azure::{client::AzureDevOpsClient, organizations};
use mcp_tools_codegen::mcp_tool;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    schemars::{self, JsonSchema},
    serde::Deserialize,
};

#[derive(Deserialize, JsonSchema)]
pub struct GetCurrentUserArgs {}

#[mcp_tool(
    name = "azdo_get_current_user",
    description = "Get current user profile"
)]
pub async fn get_current_user(
    client: &AzureDevOpsClient,
    _args: GetCurrentUserArgs,
) -> Result<CallToolResult, McpError> {
    log::info!("Tool invoked: azdo_get_current_user");
    let profile = organizations::get_profile(client)
        .await
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: e.to_string().into(),
            data: None,
        })?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);

    wtr.write_record(&[profile.display_name, profile.email_address])
        .map_err(|e| McpError {
            code: ErrorCode(-32000),
            message: format!("Failed to write CSV: {}", e).into(),
            data: None,
        })?;

    wtr.flush().map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to flush CSV: {}", e).into(),
        data: None,
    })?;

    let csv_bytes = wtr.into_inner().map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to get CSV bytes: {}", e).into(),
        data: None,
    })?;

    let data = String::from_utf8(csv_bytes).map_err(|e| McpError {
        code: ErrorCode(-32000),
        message: format!("Failed to convert CSV to string: {}", e).into(),
        data: None,
    })?;

    Ok(CallToolResult::success(vec![Content::text(data)]))
}
