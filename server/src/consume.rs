use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use futures::stream::{self, StreamExt};
use lambda_http::{Body, Error, Request, RequestExt, Response};
use serde::{Deserialize, Serialize};

const MAX_CONCURRENT_READS: usize = 10;

#[derive(Serialize, Deserialize)]
struct Message {
    offset: i64,
    payload: serde_json::Value,
    timestamp: u128,
}

#[derive(Serialize)]
struct ConsumeResponse {
    messages: Vec<Message>,
    next_offset: i64,
}

/// Handle GET /topics/{topic}/consume?offset=1&limit=10
pub async fn handle(
    event: Request,
    s3: &S3Client,
    bucket: &str,
    topic: &str,
) -> Result<Response<Body>, Error> {
    // Parse query parameters
    let params = event.query_string_parameters();
    let start_offset: i64 = params
        .first("offset")
        .and_then(|s: &str| s.parse().ok())
        .unwrap_or(1);
    let limit: i32 = params
        .first("limit")
        .and_then(|s: &str| s.parse().ok())
        .unwrap_or(10)
        .min(100); // Cap at 100

    tracing::info!(%topic, %start_offset, %limit, "Consuming messages");

    // List objects from S3
    let prefix = format!("topics/{}/", topic);
    let start_after = if start_offset > 1 {
        format!("topics/{}/{:020}.json", topic, start_offset - 1)
    } else {
        String::new()
    };

    let mut list_req = s3.list_objects_v2().bucket(bucket).prefix(&prefix).max_keys(limit);

    if !start_after.is_empty() {
        list_req = list_req.start_after(&start_after);
    }

    let list_result = list_req.send().await.map_err(|e| {
        tracing::error!(error = %e, %prefix, "Failed to list S3 objects");
        e
    })?;

    let keys: Vec<String> = list_result
        .contents()
        .iter()
        .filter_map(|obj| obj.key().map(String::from))
        .collect();

    if keys.is_empty() {
        let response = ConsumeResponse {
            messages: vec![],
            next_offset: start_offset,
        };
        return Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&response)?))?);
    }

    // Fetch messages in parallel with bounded concurrency
    let messages: Vec<Message> = stream::iter(keys)
        .map(|key| {
            let s3 = s3.clone();
            let bucket = bucket.to_string();
            async move { fetch_message(&s3, &bucket, &key).await }
        })
        .buffer_unordered(MAX_CONCURRENT_READS)
        .filter_map(|result| async { result.ok() })
        .collect()
        .await;

    // Sort by offset (buffer_unordered doesn't preserve order)
    let mut messages = messages;
    messages.sort_by_key(|m| m.offset);

    let next_offset = messages.last().map(|m| m.offset + 1).unwrap_or(start_offset);

    tracing::info!(%topic, count = messages.len(), %next_offset, "Messages consumed");

    let response = ConsumeResponse {
        messages,
        next_offset,
    };

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&response)?))?)
}

/// Fetch a single message from S3
async fn fetch_message(s3: &S3Client, bucket: &str, key: &str) -> Result<Message, Error> {
    let result = s3.get_object().bucket(bucket).key(key).send().await?;

    let bytes = result.body.collect().await?.into_bytes();
    let message: Message = serde_json::from_slice(&bytes)?;

    Ok(message)
}
