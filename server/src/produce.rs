use anyhow::Result;
use aws_sdk_dynamodb::{types::AttributeValue, types::ReturnValue, Client as DynamoClient};
use aws_sdk_s3::{primitives::ByteStream, Client as S3Client};
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct ProduceRequest {
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct ProduceResponse {
    topic: String,
    offset: i64,
}

#[derive(Serialize)]
struct Message {
    offset: i64,
    payload: serde_json::Value,
    timestamp: u128,
}

/// Handle POST /topics/{topic}/produce
pub async fn handle(
    event: Request,
    s3: &S3Client,
    dynamo: &DynamoClient,
    bucket: &str,
    table: &str,
    topic: &str,
) -> Result<Response<Body>, Error> {
    // Parse request body
    let body: ProduceRequest = match event.payload() {
        Ok(Some(b)) => b,
        Ok(None) => {
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error":"missing request body"}"#))?)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse request body");
            return Ok(Response::builder()
                .status(400)
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"error":"invalid json: {}"}}"#, e)))?)
        }
    };

    // 1. Atomically get next offset from DynamoDB
    let offset = next_offset(dynamo, table, topic).await?;

    // 2. Create message
    let message = Message {
        offset,
        payload: body.payload,
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    };

    // 3. Write to S3
    let s3_key = format!("topics/{}/{:020}.json", topic, offset);
    let message_bytes = serde_json::to_vec(&message)?;

    s3.put_object()
        .bucket(bucket)
        .key(&s3_key)
        .body(ByteStream::from(message_bytes))
        .content_type("application/json")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %s3_key, "Failed to write to S3");
            e
        })?;

    tracing::info!(%topic, %offset, %s3_key, "Message produced");

    // 4. Return response
    let response = ProduceResponse {
        topic: topic.to_string(),
        offset,
    };

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&response)?))?)
}

/// Atomically increment topic offset and return the new value
async fn next_offset(client: &DynamoClient, table: &str, topic: &str) -> Result<i64, Error> {
    let result = client
        .update_item()
        .table_name(table)
        .key("pk", AttributeValue::S(topic.to_string()))
        .update_expression("SET current_offset = if_not_exists(current_offset, :zero) + :inc")
        .expression_attribute_values(":zero", AttributeValue::N("0".into()))
        .expression_attribute_values(":inc", AttributeValue::N("1".into()))
        .return_values(ReturnValue::UpdatedNew)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %topic, "Failed to increment offset in DynamoDB");
            e
        })?;

    let offset = result
        .attributes()
        .and_then(|attrs| attrs.get("current_offset"))
        .and_then(|v| v.as_n().ok())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);

    Ok(offset)
}

