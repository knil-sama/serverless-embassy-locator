# serverless-embassy-locator

## Purpose

## Architecture

clean_embassy: lambda in Rust that will be trigger by a new file pushed on s3 then clean the csv and push to another s3 folder in parquet

## Stack

### Rust
Used this extension for facilitate lambda and rust integration https://github.com/awslabs/aws-lambda-rust-runtime

We setup workspace so shared dependency are below root [Cargo.toml](./Cargo.toml)

`cargo test`

### SAM

s3 ressource aren't created by SAM because it's tricky to delete or reuse them with it so they were created by hand

Everything else is done using SAM, policy for tag are handled in another repository


## Deployment

Build lamda

`cargo lambda build --release --target=x86_64-unknown-linux-gnu --features vendored`

`sam deploy --profile admin --stack-name serverless-embassy --capabilities CAPABILITY_IAM --s3-bucket cdemonchy-eu-west-3-aws-sam --s3-prefix serverless-embassy --region eu-west-3`

# TODO

fix current csv when crawling or cleaning

use env variable for bucket name