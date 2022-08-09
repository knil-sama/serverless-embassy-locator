#!/usr/bin/env sh
set -x
set -e
wget https://github.com/database-of-embassies/database-of-embassies/raw/master/database_of_embassies.csv --output-document=data/embassie.csv
