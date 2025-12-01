use crate::azure::client::{AzureDevOpsClient, AzureError};
use reqwest::Method;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    #[serde(rename = "defaultValue")]
    pub default_value: Option<String>, // This is the team's default area path
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamListResponse {
    pub value: Vec<Team>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemType {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<serde_json::Value>, // Icon can be an object, not just a string
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    #[serde(rename = "referenceName")]
    pub reference_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemTypeListResponse {
    pub value: Vec<WorkItemType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardColumn {
    pub id: String,
    pub name: String,
    #[serde(rename = "itemLimit")]
    pub item_limit: i32,
    #[serde(rename = "stateMappings")]
    pub state_mappings: serde_json::Value,
    #[serde(rename = "columnType")]
    pub column_type: String,
    #[serde(default)]
    #[serde(rename = "isSplit")]
    pub is_split: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardRow {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardField {
    #[serde(rename = "referenceName")]
    pub reference_name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardFields {
    #[serde(rename = "columnField")]
    pub column_field: BoardField,
    #[serde(rename = "rowField")]
    pub row_field: BoardField,
    #[serde(rename = "doneField")]
    pub done_field: BoardField,
}

/// Summary information for a board (used in list operations)
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardSummary {
    pub id: String,
    pub name: String,
    pub url: String,
}

/// Detailed board information (used in get operations)
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardDetail {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub revision: Option<i32>,
    #[serde(default)]
    pub columns: Option<Vec<BoardColumn>>,
    #[serde(default)]
    pub rows: Option<Vec<BoardRow>>,
    #[serde(default)]
    #[serde(rename = "isValid")]
    pub is_valid: Option<bool>,
    #[serde(default)]
    #[serde(rename = "allowedMappings")]
    pub allowed_mappings: Option<serde_json::Value>,
    #[serde(default)]
    #[serde(rename = "canEdit")]
    pub can_edit: Option<bool>,
    #[serde(default)]
    pub fields: Option<BoardFields>,
    // Skipping _links as it's not needed
}

impl BoardDetail {
    /// Extract work item types from the board's allowed mappings
    pub fn get_work_item_types(&self) -> Vec<String> {
        let mut types = Vec::new();

        if let Some(mappings) = &self.allowed_mappings
            && let Some(obj) = mappings.as_object()
        {
            for (_column_type, type_mappings) in obj {
                if let Some(type_obj) = type_mappings.as_object() {
                    for (work_item_type, _states) in type_obj {
                        if !types.contains(work_item_type) {
                            types.push(work_item_type.clone());
                        }
                    }
                }
            }
        }

        types
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardListResponse {
    pub value: Vec<BoardSummary>,
}

/// List all teams in the project
pub async fn list_teams(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
) -> Result<Vec<Team>, AzureError> {
    // Teams API: https://dev.azure.com/{organization}/_apis/projects/{project}/teams
    let path = format!("projects/{}/teams?api-version=7.1", project);
    let response: TeamListResponse = client
        .org_request(organization, Method::GET, &path, None::<&String>)
        .await?;

    Ok(response.value)
}

/// Get a specific team by ID or name
pub async fn get_team(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
) -> Result<Team, AzureError> {
    // Team API: https://dev.azure.com/{organization}/_apis/projects/{project}/teams/{teamId}
    let path = format!("projects/{}/teams/{}?api-version=7.1", project, team_id);
    client
        .org_request(organization, Method::GET, &path, None::<&String>)
        .await
}

/// List all work item types (Stories, Epics, Features, Bugs, etc.)
pub async fn list_work_item_types(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
) -> Result<Vec<WorkItemType>, AzureError> {
    let path = "wit/workitemtypes?api-version=7.1";
    let response: WorkItemTypeListResponse = client.get(organization, project, path).await?;
    Ok(response.value)
}

/// List boards for a specific team
/// Note: In Azure DevOps, boards are team-specific Kanban boards
pub async fn list_boards(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
) -> Result<Vec<BoardSummary>, AzureError> {
    // Team-specific boards: https://dev.azure.com/{org}/{project}/{team}/_apis/work/boards
    let path = "work/boards?api-version=7.1";
    let response: BoardListResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            path,
            None::<&String>,
        )
        .await?;
    Ok(response.value)
}

/// Get a specific board (requires team context)
pub async fn get_board(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
    board_id: &str,
) -> Result<BoardDetail, AzureError> {
    // Team-specific board: https://dev.azure.com/{org}/{project}/{team}/_apis/work/boards/{boardId}
    let path = format!("work/boards/{}?api-version=7.1", board_id);
    client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            &path,
            None::<&String>,
        )
        .await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardColumnsResponse {
    pub value: Vec<BoardColumn>,
}

/// List columns for a specific board
pub async fn list_board_columns(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
    board_id: &str,
) -> Result<Vec<BoardColumn>, AzureError> {
    // Board columns: https://dev.azure.com/{org}/{project}/{team}/_apis/work/boards/{board}/columns
    let path = format!("work/boards/{}/columns?api-version=7.1", board_id);
    let response: BoardColumnsResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            &path,
            None::<&String>,
        )
        .await?;
    Ok(response.value)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardRowsResponse {
    pub value: Vec<BoardRow>,
}

/// List rows (swimlanes) for a specific board
pub async fn list_board_rows(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    team_id: &str,
    board_id: &str,
) -> Result<Vec<BoardRow>, AzureError> {
    // Board rows: https://dev.azure.com/{org}/{project}/{team}/_apis/work/boards/{board}/rows
    let path = format!("work/boards/{}/rows?api-version=7.1", board_id);
    let response: BoardRowsResponse = client
        .team_request(
            organization,
            project,
            Method::GET,
            team_id,
            &path,
            None::<&String>,
        )
        .await?;
    Ok(response.value)
}
