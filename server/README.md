# Serverless Streams - Server

A Rust Lambda function that implements a serverless message streaming API, hosted behind AWS API Gateway.

## Architecture

```
API Gateway → Lambda (this) → S3 (messages) + DynamoDB (counters)
```

## Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/topics/{topic}/produce` | Produce a message to a topic |
| `GET` | `/topics/{topic}/consume?offset=1&limit=10` | Consume messages from a topic |
| `GET` | `/health` | Health check |

### Produce Request

```bash
curl -X POST http://localhost:9000/topics/orders/produce \
  -H "Content-Type: application/json" \
  -d '{"payload": {"item": "book", "quantity": 2}}'
```

Response:
```json
{"topic": "orders", "offset": 1}
```

### Consume Request

```bash
curl "http://localhost:9000/topics/orders/consume?offset=1&limit=10"
```

Response:
```json
{
  "messages": [
    {"offset": 1, "payload": {"item": "book", "quantity": 2}, "timestamp": 1702000000000}
  ],
  "next_offset": 2
}
```

## Local Development

### Prerequisites

- Rust 1.70+
- [cargo-lambda](https://www.cargo-lambda.info/)
- AWS credentials configured (for S3 and DynamoDB access)

### Setup

1. Deploy the data stack (S3 + DynamoDB):
   ```bash
   cd ../infra
   npx cdk deploy ServerlessStreamsData
   ```

2. Source environment variables:
   ```bash
   cd server
   cp env.dev.example .env.dev
   # Edit .env.dev with your account ID
   source .env.dev
   ```

3. Run locally:
   ```bash
   cargo lambda watch
   ```

4. Test:
   ```bash
   # Produce a message
   curl -X POST http://localhost:9000/topics/test/produce \
     -H "Content-Type: application/json" \
     -d '{"payload": {"hello": "world"}}'

   # Consume messages
   curl "http://localhost:9000/topics/test/consume?offset=1&limit=10"
   ```

## Building for Deployment

```bash
cargo lambda build --release --arm64
```

The binary will be at `target/lambda/server/bootstrap`.

**Note**: This binary must be built before deploying the API stack, as CDK references it.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `BUCKET_NAME` | S3 bucket for storing messages |
| `COUNTERS_TABLE` | DynamoDB table for offset counters |
| `AWS_REGION` | AWS region (default from AWS config) |
| `RUST_LOG` | Log level (e.g., `info`, `debug`) |

## How It Works

### Produce Flow
1. Parse request body `{"payload": {...}}`
2. Atomically increment offset counter in DynamoDB (using `UpdateItem` with `ADD`)
3. Write message to S3: `topics/{topic}/{offset:020}.json`
4. Return `{"topic", "offset"}`

### Consume Flow
1. Parse query params `?offset=X&limit=Y`
2. List S3 objects from offset onwards
3. Fetch messages in parallel (max 10 concurrent reads)
4. Sort by offset (since parallel reads don't preserve order)
5. Return messages + next_offset

## Performance

- **Cold start**: ~90-100ms (Rust + ARM64)
- **Warm invocation**: ~2-5ms
- **Parallel S3 reads**: Up to 10 concurrent (bounded to avoid rate limits)

## TODO

- [ ] Add client SDK (Python, TypeScript, etc.) for easier integration
- [ ] Add consumer groups for offset tracking
- [ ] Add message batching for better S3 write efficiency
- [ ] Add compression for large messages
