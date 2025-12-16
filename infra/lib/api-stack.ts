import * as cdk from "aws-cdk-lib";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as apigateway from "aws-cdk-lib/aws-apigateway";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as logs from "aws-cdk-lib/aws-logs";
import { Construct } from "constructs";
import * as path from "path";
import { getEntityName, PROJECT_PREFIX } from "./utils";

interface ApiStackProps extends cdk.StackProps {
  bucket: s3.IBucket;
  countersTable: dynamodb.ITable;
}

export class ApiStack extends cdk.Stack {
  public readonly api: apigateway.RestApi;
  public readonly lambda: lambda.Function;

  constructor(scope: Construct, id: string, props: ApiStackProps) {
    super(scope, id, props);

    const { bucket, countersTable } = props;

    // Lambda function (Rust, ARM64)
    this.lambda = new lambda.Function(this, "StreamsFunction", {
      functionName: getEntityName(this, `${PROJECT_PREFIX}-server`),
      runtime: lambda.Runtime.PROVIDED_AL2023,
      architecture: lambda.Architecture.ARM_64,
      handler: "bootstrap",
      code: lambda.Code.fromAsset(
        path.join(__dirname, "../../server/target/lambda/server")
      ),
      memorySize: 256,
      timeout: cdk.Duration.seconds(30),
      environment: {
        BUCKET_NAME: bucket.bucketName,
        COUNTERS_TABLE: countersTable.tableName,
        RUST_LOG: "info",
      },
      logRetention: logs.RetentionDays.ONE_WEEK,
    });

    // Grant permissions
    bucket.grantReadWrite(this.lambda);
    countersTable.grantReadWriteData(this.lambda);

    // API Gateway
    this.api = new apigateway.RestApi(this, "StreamsApi", {
      restApiName: getEntityName(this, `${PROJECT_PREFIX}-api`),
      description: "Serverless Streams API",
      deployOptions: {
        stageName: "v1",
        throttlingBurstLimit: 1000,
        throttlingRateLimit: 500,
      },
      defaultCorsPreflightOptions: {
        allowOrigins: apigateway.Cors.ALL_ORIGINS,
        allowMethods: apigateway.Cors.ALL_METHODS,
        allowHeaders: ["Content-Type", "Authorization"],
      },
    });

    // Lambda integration
    const lambdaIntegration = new apigateway.LambdaIntegration(this.lambda);

    // Routes: /topics/{topic}/produce and /topics/{topic}/consume
    const topics = this.api.root.addResource("topics");
    const topic = topics.addResource("{topic}");

    // POST /topics/{topic}/produce
    const produce = topic.addResource("produce");
    produce.addMethod("POST", lambdaIntegration);

    // GET /topics/{topic}/consume
    const consume = topic.addResource("consume");
    consume.addMethod("GET", lambdaIntegration);

    // GET /health
    const health = this.api.root.addResource("health");
    health.addMethod("GET", lambdaIntegration);

    // Outputs
    new cdk.CfnOutput(this, "ApiUrl", {
      value: this.api.url,
      exportName: "ServerlessStreamsApiUrl",
    });

    new cdk.CfnOutput(this, "ProduceEndpoint", {
      value: `${this.api.url}topics/{topic}/produce`,
      description: "POST endpoint to produce messages",
    });

    new cdk.CfnOutput(this, "ConsumeEndpoint", {
      value: `${this.api.url}topics/{topic}/consume?offset=1&limit=10`,
      description: "GET endpoint to consume messages",
    });

    new cdk.CfnOutput(this, "LambdaFunctionName", {
      value: this.lambda.functionName,
      exportName: "ServerlessStreamsLambdaName",
    });
  }
}

