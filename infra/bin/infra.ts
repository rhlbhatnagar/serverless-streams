#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import { DataStack } from "../lib/data-stack";
import { ApiStack } from "../lib/api-stack";

const app = new cdk.App();

// Data Stack (S3 + DynamoDB)
const dataStack = new DataStack(app, "ServerlessStreamsData", {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  description: "Serverless Streams - Data layer (S3 + DynamoDB)",
});

// API Stack (API Gateway + Lambda)
const apiStack = new ApiStack(app, "ServerlessStreamsApi", {
  bucket: dataStack.bucket,
  countersTable: dataStack.countersTable,
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  description: "Serverless Streams - API layer (API Gateway + Lambda)",
});

// Ensure API stack depends on Data stack
apiStack.addDependency(dataStack);

// Tags for all resources
cdk.Tags.of(dataStack).add("Project", "serverless-streams");
cdk.Tags.of(dataStack).add("Environment", "dev");
cdk.Tags.of(apiStack).add("Project", "serverless-streams");
cdk.Tags.of(apiStack).add("Environment", "dev");
