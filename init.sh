#!/usr/bin/env sh
set -x
set -e
BUCKET_NAME=raw-embassies
aws s3api head-bucket --bucket ${BUCKET_NAME}  2>&1
if [ $? -ne 0 ]
then
    aws s3api create-bucket \
        --no-object-lock-enabled-for-bucket \
        --acl private \
        --bucket $BUCKET_NAME \
        --create-bucket-configuration '{"LocationConstraint": "eu-west-3"}'
fi
ACCOUNT_ID=`aws sts get-caller-identity | jq .Account --raw-output`
aws s3control create-access-point \
    --account-id ${ACCOUNT_ID} \
    --name raw-bucket-access \
    --bucket ${BUCKET_NAME}
