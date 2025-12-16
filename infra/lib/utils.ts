import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";

/**
 * Generates a unique entity name by appending the AWS account ID.
 * This ensures globally unique names for resources like S3 buckets.
 *
 * @param scope - The CDK construct scope (to get account info)
 * @param entityName - The base name for the entity
 * @returns Formatted name: `{entityName}-{accountId}`
 */
export function getEntityName(scope: Construct, entityName: string): string {
  const stack = cdk.Stack.of(scope);
  const accountId = stack.account;

  // Convert entity name to lowercase and replace spaces with hyphens
  const formattedEntityName = entityName.toLowerCase().replace(/\s+/g, "-");

  return `${formattedEntityName}-${accountId}`;
}

/**
 * Project prefix for consistent naming across all resources.
 */
export const PROJECT_PREFIX = "sls-streams";

