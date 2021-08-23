#!/usr/bin/env sh
set -x
set -e
BUCKET_NAME=raw-embassies
ACCOUNT_ID=`aws sts get-caller-identity | jq .Account --raw-output`
aws s3control create-access-point --account-id ${ACCOUNT_ID} --name ${BUCKET_NAME}-ap --bucket ${BUCKET_NAME}
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
