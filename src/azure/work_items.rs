use crate::azure::client::{AzureDevOpsClient, AzureError};
use crate::azure::models::{
    Comment, CommentListResponse, WiqlQuery, WiqlResponse, WorkItem, WorkItemListResponse,
};
use futures::future::join_all;
use serde::Serialize;
use serde_json::Value;

const COMMENT_FETCH_CONCURRENCY: usize = 10;

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

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
    include_latest_n_comments: Option<i32>,
) -> Result<Option<WorkItem>, AzureError> {
    let result = get_work_items(
        client,
        organization,
        project,
        &[id],
        include_latest_n_comments,
    )
    .await;

    match result {
        Ok(items) => Ok(items.into_iter().next()),
        Err(AzureError::ApiError(msg))
            if msg.contains("WorkItemUnauthorizedAccessException")
                || msg.contains("Work item does not exist") =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

pub async fn get_comments(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    work_item_id: u32,
    n: i32,
) -> Result<Vec<Comment>, AzureError> {
    // Early return for n=0 case
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut all_comments = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut path = format!(
            "wit/workitems/{}/comments?api-version=7.1-preview.3&order=desc",
            work_item_id
        );

        // Only set $top on the first request (not with continuation tokens)
        if n > 0 && continuation_token.is_none() {
            path.push_str(&format!("&$top={}", n));
        }

        if let Some(token) = &continuation_token {
            path.push_str(&format!("&continuationToken={}", urlencoding::encode(token)));
        }

        let (response, headers): (CommentListResponse, _) = client
            .get_with_headers(organization, project, &path)
            .await?;

        all_comments.extend(response.comments);

        // Extract continuation token from headers
        continuation_token = headers
            .get("x-ms-continuationtoken")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Break if no continuation token or if we've fetched enough comments
        if continuation_token.is_none() {
            break;
        }

        if n != -1 && all_comments.len() >= n as usize {
            break;
        }
    }

    Ok(all_comments)
}

pub async fn get_work_items(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    ids: &[u32],
    include_latest_n_comments: Option<i32>,
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

    if let Some(n) = include_latest_n_comments {
        let ids: Vec<(usize, u32)> = all_work_items
            .iter()
            .enumerate()
            .map(|(i, wi)| (i, wi.id))
            .collect();

        for chunk in ids.chunks(COMMENT_FETCH_CONCURRENCY) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|&(_, id)| get_comments(client, organization, project, id, n))
                .collect();

            let results = join_all(futures).await;

            for (&(i, _), result) in chunk.iter().zip(results) {
                all_work_items[i].comments = Some(result?);
            }
        }
    }

    Ok(all_work_items)
}

pub async fn create_work_item(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    work_item_type: &str,
    fields: &[(String, Value)],
    multiline_fields_format: &[(String, String)],
) -> Result<WorkItem, AzureError> {
    let mut operations: Vec<JsonPatchOperation> = Vec::new();

    for (field, format) in multiline_fields_format {
        operations.push(JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/multilineFieldsFormat/{}", escape_json_pointer_token(field)),
            value: Some(Value::String(format.to_string())),
            from: None,
        });
    }

    for (k, v) in fields {
        operations.push(JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/fields/{}", escape_json_pointer_token(k)),
            value: Some(v.clone()),
            from: None,
        });
    }

    let path = format!("wit/workitems/${}?api-version=7.1", urlencoding::encode(work_item_type));
    client
        .post_patch(organization, project, &path, &operations)
        .await
}

pub async fn update_work_item(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    id: u32,
    fields: &[(String, Value)],
    multiline_fields_format: &[(String, String)],
) -> Result<WorkItem, AzureError> {
    let mut operations: Vec<JsonPatchOperation> = Vec::new();

    for (field, format) in multiline_fields_format {
        operations.push(JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/multilineFieldsFormat/{}", escape_json_pointer_token(field)),
            value: Some(Value::String(format.to_string())),
            from: None,
        });
    }

    for (field, value) in fields {
        operations.push(JsonPatchOperation {
            op: "add".to_string(),
            path: format!("/fields/{}", escape_json_pointer_token(field)),
            value: Some(value.clone()),
            from: None,
        });
    }

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
    format: &str,
) -> Result<Value, AzureError> {
    let encoded_format = urlencoding::encode(format);
    let path = format!(
        "wit/workitems/{}/comments?api-version=7.2-preview.4&format={}",
        work_item_id, encoded_format
    );
    let body = serde_json::json!({
        "text": text
    });
    client.post(organization, project, &path, &body).await
}

pub async fn update_comment(
    client: &AzureDevOpsClient,
    organization: &str,
    project: &str,
    work_item_id: u32,
    comment_id: u32,
    text: &str,
    format: &str,
) -> Result<Value, AzureError> {
    let encoded_format = urlencoding::encode(format);
    let path = format!(
        "wit/workitems/{}/comments/{}?api-version=7.2-preview.4&format={}",
        work_item_id, comment_id, encoded_format
    );
    let body = serde_json::json!({
        "text": text
    });
    client.patch(organization, project, &path, &body).await
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
    include_latest_n_comments: Option<i32>,
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
    get_work_items(
        client,
        organization,
        project,
        &ids,
        include_latest_n_comments,
    )
    .await
}
