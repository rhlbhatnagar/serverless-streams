# Serverless Streams - Infrastructure

AWS CDK infrastructure code for deploying the Serverless Streams platform.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  API Stack (ServerlessStreamsApi)                           │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  API Gateway                                          │ │
│  │  POST /topics/{topic}/produce                         │ │
│  │  GET  /topics/{topic}/consume                         │ │
│  └──────────────┬────────────────────────────────────────┘ │
│                 │                                            │
│                 ▼                                            │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  Lambda Function (Rust, ARM64)                       │ │
│  └──────────────┬────────────────────────────────────────┘ │
└─────────────────┼───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│  Data Stack (ServerlessStreamsData)                         │
│  ┌──────────────────┐  ┌────────────────────────────────┐  │
│  │  S3 Bucket       │  │  DynamoDB Table                │  │
│  │  (messages)      │  │  (offset counters)             │  │
│  └──────────────────┘  └────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Stacks

### ServerlessStreamsData
- **S3 Bucket**: Stores messages as JSON files (`topics/{topic}/{offset}.json`)
- **DynamoDB Table**: Serverless/on-demand table for atomic offset counters
- **Lifecycle**: 30-day message retention (configurable)

### ServerlessStreamsApi
- **API Gateway**: REST API with CORS enabled
- **Lambda Function**: Rust runtime, ARM64 architecture
- **Permissions**: Read/write access to S3 and DynamoDB

## Prerequisites

- Node.js 23+
- AWS CDK CLI (`npm install -g aws-cdk`)
- AWS credentials configured
- Rust Lambda binary built (see `../server/README.md`)

## Deployment

### 1. Deploy Data Stack

```bash
cd infra
npx cdk deploy ServerlessStreamsData
```

This creates:
- S3 bucket: `sls-streams-messages-{ACCOUNT_ID}`
- DynamoDB table: `sls-streams-counters-{ACCOUNT_ID}`

### 2. Build Lambda

```bash
cd ../server
cargo lambda build --release --arm64
```

### 3. Deploy API Stack

```bash
cd ../infra
npx cdk deploy ServerlessStreamsApi
```

This creates:
- API Gateway REST API
- Lambda function
- IAM roles and permissions

## Useful Commands

```bash
# Synthesize CloudFormation templates
npx cdk synth

# Deploy a specific stack
npx cdk deploy ServerlessStreamsData
npx cdk deploy ServerlessStreamsApi

# Deploy all stacks
npx cdk deploy --all

# View differences
npx cdk diff

# Destroy stacks (careful!)
npx cdk destroy ServerlessStreamsApi
npx cdk destroy ServerlessStreamsData
```

## Configuration

### Resource Naming

Resources are named using `getEntityName()` utility:
- Format: `{prefix}-{resource}-{account-id}`
- Ensures globally unique names (especially for S3 buckets)

### Environment Variables

Lambda receives these environment variables:
- `BUCKET_NAME`: S3 bucket name
- `COUNTERS_TABLE`: DynamoDB table name
- `RUST_LOG`: Log level (default: `info`)

## Outputs

After deployment, CDK outputs:
- `ApiUrl`: Base URL for the API Gateway
- `ProduceEndpoint`: Full produce endpoint URL
- `ConsumeEndpoint`: Full consume endpoint URL
- `LambdaFunctionName`: Lambda function name

## Cost Considerations

- **S3**: Pay per GB stored + requests
- **DynamoDB**: On-demand billing (pay per request)
- **Lambda**: Pay per invocation + compute time
- **API Gateway**: Pay per API call

All resources scale to zero when idle (except S3 storage).
