import * as cdk from "aws-cdk-lib";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import { Construct } from "constructs";
import { getEntityName, PROJECT_PREFIX } from "./utils";

export class DataStack extends cdk.Stack {
  public readonly bucket: s3.Bucket;
  public readonly countersTable: dynamodb.Table;

  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // S3 Bucket for messages
    this.bucket = new s3.Bucket(this, "MessagesBucket", {
      bucketName: getEntityName(this, `${PROJECT_PREFIX}-messages`),
      removalPolicy: cdk.RemovalPolicy.DESTROY, // For dev - change to RETAIN for prod
      autoDeleteObjects: true, // For dev - remove for prod
      encryption: s3.BucketEncryption.S3_MANAGED,
      blockPublicAccess: s3.BlockPublicAccess.BLOCK_ALL,
      lifecycleRules: [
        {
          id: "ExpireOldMessages",
          enabled: true,
          expiration: cdk.Duration.days(30),
        },
      ],
    });

    // DynamoDB Table for offset counters (serverless/on-demand)
    this.countersTable = new dynamodb.Table(this, "CountersTable", {
      tableName: getEntityName(this, `${PROJECT_PREFIX}-counters`),
      partitionKey: {
        name: "pk",
        type: dynamodb.AttributeType.STRING,
      },
      billingMode: dynamodb.BillingMode.PAY_PER_REQUEST, // Serverless!
      removalPolicy: cdk.RemovalPolicy.DESTROY, // For dev - change to RETAIN for prod
      pointInTimeRecoverySpecification: {
        pointInTimeRecoveryEnabled: true,
      },
    });

    // Outputs
    new cdk.CfnOutput(this, "BucketName", {
      value: this.bucket.bucketName,
      exportName: "ServerlessStreamsBucketName",
    });

    new cdk.CfnOutput(this, "BucketArn", {
      value: this.bucket.bucketArn,
      exportName: "ServerlessStreamsBucketArn",
    });

    new cdk.CfnOutput(this, "CountersTableName", {
      value: this.countersTable.tableName,
      exportName: "ServerlessStreamsCountersTable",
    });

    new cdk.CfnOutput(this, "CountersTableArn", {
      value: this.countersTable.tableArn,
      exportName: "ServerlessStreamsCountersTableArn",
    });
  }
}
