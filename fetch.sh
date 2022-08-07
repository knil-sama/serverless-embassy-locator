#!/usr/bin/env sh
set -x
set -e
wget https://www.data.gouv.fr/en/datasets/r/b629598e-df53-4915-b3c2-e70743c7fc34 --output-document=data/embassies_french.csv
