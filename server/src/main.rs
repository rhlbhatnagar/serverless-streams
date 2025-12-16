use anyhow::Result;
use aws_sdk_dynamodb::Client as DynamoClient;
use aws_sdk_s3::Client as S3Client;
use lambda_http::{http::Method, run, service_fn, Body, Error, Request, Response};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod consume;
mod produce;

/// Shared state across Lambda invocations (connection pooling)
struct AppState {
    s3: S3Client,
    dynamo: DynamoClient,
    bucket: String,
    table: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Load AWS config (uses env vars or IAM role)
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    // Create shared state (reused across warm invocations)
    let state = Arc::new(AppState {
        s3: S3Client::new(&config),
        dynamo: DynamoClient::new(&config),
        bucket: std::env::var("BUCKET_NAME").expect("BUCKET_NAME must be set"),
        table: std::env::var("COUNTERS_TABLE").expect("COUNTERS_TABLE must be set"),
    });

    tracing::info!(
        bucket = %state.bucket,
        table = %state.table,
        "Lambda initialized"
    );

    // Run the Lambda handler
    run(service_fn(|event: Request| {
        let state = Arc::clone(&state);
        async move { router(event, state).await }
    }))
    .await
}

/// Route requests to appropriate handler based on method and path
async fn router(event: Request, state: Arc<AppState>) -> Result<Response<Body>, Error> {
    let method = event.method().clone();
    let path = event.uri().path().to_string();

    tracing::info!(%method, %path, "Incoming request");

    // Parse path and skip API Gateway stage prefix (e.g., /v1/)
    let parts: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .skip_while(|s| s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit()))
        .collect();

    match (method, parts.as_slice()) {
        // POST /topics/{topic}/produce
        (Method::POST, ["topics", topic, "produce"]) => {
            let topic = topic.to_string();
            produce::handle(event, &state.s3, &state.dynamo, &state.bucket, &state.table, &topic)
                .await
        }

        // GET /topics/{topic}/consume
        (Method::GET, ["topics", topic, "consume"]) => {
            let topic = topic.to_string();
            consume::handle(event, &state.s3, &state.bucket, &topic).await
        }

        // Health check
        (Method::GET, ["health"]) => Ok(Response::builder()
            .status(200)
            .body(Body::from(r#"{"status":"ok"}"#))?),

        // Not found
        _ => Ok(Response::builder()
            .status(404)
            .body(Body::from(r#"{"error":"not found"}"#))?),
    }
}
