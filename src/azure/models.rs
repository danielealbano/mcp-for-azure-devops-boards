use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardListResponse {
    pub count: u32,
    pub value: Vec<BoardReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardReference {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemListResponse {
    pub count: u32,
    pub value: Vec<WorkItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: u32,
    pub fields: HashMap<String, serde_json::Value>,
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<Comment>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentListResponse {
    #[serde(rename = "totalCount")]
    pub total_count: u32,
    #[serde(rename = "count")]
    pub count: u32,
    #[serde(rename = "comments")]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub id: u32,
    pub text: String,
    #[serde(rename = "createdDate")]
    pub created_date: String,
    #[serde(rename = "createdBy")]
    pub created_by: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WiqlQuery {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WiqlResponse {
    #[serde(rename = "workItems")]
    pub work_items: Vec<WorkItemReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemReference {
    pub id: u32,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardColumn {
    pub id: String,
    pub name: String,
    #[serde(rename = "itemLimit")]
    pub item_limit: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub columns: Vec<BoardColumn>,
    // Add swimlanes if needed
}
