#!/usr/bin/env sh
set -x
set -e
BUCKET_NAME=raw-embassies
ACCOUNT_ID=`aws sts get-caller-identity | jq .Account --raw-output`
# https://docs.aws.amazon.com/cli/latest/reference/s3control/create-access-point.html
aws s3control create-access-point --account-id ${ACCOUNT_ID} --name ${BUCKET_NAME}-ap --bucket ${BUCKET_NAME}
# https://docs.aws.amazon.com/cli/latest/reference/s3control/create-access-point-for-object-lambda.html
aws s3control create-access-point-for-object-lambda --account-id ${ACCOUNT_ID} --name ${BUCKET_NAME}-ap-fol
--configuration '{"SupportingAccessPoint": "${BUCKET_NAME}-ap","CloudWatchMetricsEnabled": true,"AllowedFeatures": ["GetObject-Range","GetObject-PartNumber"],"TransformationConfigurations": [
                {
                          "Actions": ["GetObject", ...],
                                "ContentTransformation": {
                                        "AwsLambda": {
                                                  "FunctionArn": "string",
                                                            "FunctionPayload": "string"
                                                                    }
                                                                      }
                                                                      }
                                                                      ...
                                                                        ]
                                                                    }
