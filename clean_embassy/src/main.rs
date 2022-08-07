use aws_lambda_events::event::s3::S3Event;use lambda_runtime::{run, service_fn, Error, LambdaEvent};

use serde::Deserialize;
use csv::{ReaderBuilder};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct FrenchEmbassy {
    name: String, /// name mean nothing in this context
    country: String,
    latitude: f32,
    longitude: f32,
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<S3Event>) -> Result<Value, Error> {
    // Extract some useful information from the request
    for event_record in event.payload.records {
        println!("{:?}", event_record.s3)
    };
    // No extra configuration is needed as long as your Lambda has
    // the necessary permissions attached to its role.
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);


    Ok(json!({ "message": format!("Hello, {}!", "first_name") }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // disable printing the name of the module in every log line.
        .with_target(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
