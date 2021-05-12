#!/usr/bin/env sh
set -x
set -e
aws s3 sync --include "*.csv" --exclude "*.md" ./data s3://raw-embassies
