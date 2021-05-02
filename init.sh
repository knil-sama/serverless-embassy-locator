#!/usr/bin/env sh
aws s3api create-bucket \
 --no-object-lock-enabled-for-bucket \ # low update are expected of this project so better not had this
 --acl private \ # private for now
 --bucket raw-embassies \ # name of bucket
 --create-bucket-configuration '{"LocationConstraint": "eu-west-3"}' # we want this to be in Paris for the moment
