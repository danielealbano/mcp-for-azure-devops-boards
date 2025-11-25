use crate::azure::client::{AzureDevOpsClient, AzureError};
use crate::azure::models::{WiqlQuery, WiqlResponse, WorkItem, WorkItemListResponse};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct JsonPatchOperation {
    pub op: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

pub async fn get_work_item(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    id: u32,
) -> Result<WorkItem, AzureError> {
    let path = format!("wit/workitems/{}?api-version=7.1", id);
    client.get(organization, project, &path).await
}

pub async fn get_work_items(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    ids: &[u32],
) -> Result<Vec<WorkItem>, AzureError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }

    let max_items = 1000;
    let ids_to_fetch = if ids.len() > max_items {
        log::warn!(
            "Requested {} work items, limiting to {} items",
            ids.len(),
            max_items
        );
        &ids[..max_items]
    } else {
        ids
    };

    let batch_size = 200;
    let mut all_work_items = Vec::new();

    for chunk in ids_to_fetch.chunks(batch_size) {
        let ids_str = chunk
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("wit/workitems?ids={}&api-version=7.1", ids_str);
        let response: WorkItemListResponse = client.get(organization, project, &path).await?;
        all_work_items.extend(response.value);
    }

    Ok(all_work_items)
}

pub async fn create_work_item(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    work_item_type: &str,
    fields: &[(&str, Value)],
) -> Result<WorkItem, AzureError> {
    let operations: Vec<JsonPatchOperation> = fields
        .iter()
        .map(|(k, v)| JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/fields/{}", k),
            value: Some(v.clone()),
            from: None,
        })
        .collect();

    let path = format!("wit/workitems/${}?api-version=7.1", work_item_type);
    client
        .post_patch(organization, project, &path, &operations)
        .await
}

pub async fn update_work_item(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    id: u32,
    fields: &[(&str, Value)],
) -> Result<WorkItem, AzureError> {
    let operations: Vec<JsonPatchOperation> = fields
        .iter()
        .map(|(field, value)| JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/fields/{}", field),
            value: Some(value.clone()),
            from: None,
        })
        .collect();

    let path = format!("wit/workitems/{}?api-version=7.1", id);
    client
        .patch_patch(organization, project, &path, &operations)
        .await
}

pub async fn add_comment(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    work_item_id: u32,
    text: &str,
) -> Result<Value, AzureError> {
    let path = format!(
        "wit/workitems/{}/comments?api-version=7.1-preview.3",
        work_item_id
    );
    let body = serde_json::json!({
        "text": text
    });
    client.post(organization, project, &path, &body).await
}

pub async fn link_work_items(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    source_id: u32,
    target_id: u32,
    link_type: &str,
) -> Result<Value, AzureError> {
    let operations = vec![JsonPatchOperation {
        op: "add".to_string(),
        path: "/relations/-".to_string(),
        value: Some(serde_json::json!({
            "rel": link_type,
            "url": format!("https://dev.azure.com/_apis/wit/workitems/{}", target_id),
        })),
        from: None,
    }];

    let path = format!("wit/workitems/{}?api-version=7.1", source_id);
    client
        .patch_patch(organization, project, &path, &operations)
        .await
}

pub async fn query_work_items(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    query: &str,
) -> Result<Vec<WorkItem>, AzureError> {
    let wiql = WiqlQuery {
        query: query.to_string(),
    };
    let response: WiqlResponse = client
        .post(organization, project, "wit/wiql?api-version=7.1", &wiql)
        .await?;

    if response.work_items.is_empty() {
        return Ok(vec![]);
    }

    let ids: Vec<u32> = response.work_items.iter().map(|wi| wi.id).collect();
    get_work_items(client, organization, project, &ids).await
}
