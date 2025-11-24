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
