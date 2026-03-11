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
    async fn list_teams(&self, organization: &str, project: &str) -> Result<Vec<Team>, AzureError>;
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
        parent_path: Option<String>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError>;
    async fn list_iteration_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<String>,
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
    async fn get_team_iterations(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        timeframe: Option<String>,
    ) -> Result<Vec<TeamSettingsIteration>, AzureError>;
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
    async fn list_teams(&self, organization: &str, project: &str) -> Result<Vec<Team>, AzureError> {
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
        parent_path: Option<String>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError> {
        classification_nodes::list_area_paths(
            self,
            organization,
            project,
            parent_path.as_deref(),
            depth,
        )
        .await
    }
    async fn list_iteration_paths(
        &self,
        organization: &str,
        project: &str,
        parent_path: Option<String>,
        depth: i32,
    ) -> Result<ClassificationNode, AzureError> {
        classification_nodes::list_iteration_paths(
            self,
            organization,
            project,
            parent_path.as_deref(),
            depth,
        )
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
        work_items::update_work_item(
            self,
            organization,
            project,
            id,
            fields,
            multiline_fields_format,
        )
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
        work_items::query_work_items(
            self,
            organization,
            project,
            query,
            include_latest_n_comments,
        )
        .await
    }
    async fn get_team_iterations(
        &self,
        organization: &str,
        project: &str,
        team_id: &str,
        timeframe: Option<String>,
    ) -> Result<Vec<TeamSettingsIteration>, AzureError> {
        iterations::get_team_iterations(self, organization, project, team_id, timeframe.as_deref())
            .await
    }
}
