use aws_lambda_events::event::s3::S3Event;use lambda_runtime::{run, service_fn, Error, LambdaEvent};

use serde::Deserialize;
use csv::{ReaderBuilder};
use serde_json::{json, Value};
use std::io::Cursor;

#[derive(Debug, Deserialize)]
struct Embassy {
    operator: String,
    operator_qid: String,
    jurisdictions: String,
    jurisdiction_qids: String,
    country: String,
    country_qid: String,
    city: String,
    city_qid: String,
    address: String,
    latitude: f32,
    longitude: f32,
    phone: String,
    email: String,
    website: String,
    facebook: String,
    twitter: String,
    youtube: String,
    picture: String,
    picture_author: String,
    picture_license: String,
    picture_license_url: String,
    role: String,
    role_qid: String,
    creation: String,
    qid: String
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<S3Event>) -> Result<Value, Error> {
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    // Extract some useful information from the request
    for event_record in event.payload.records {
        println!("{:?}", event_record.s3);
        let resp = s3_client
        .get_object()
        .bucket(event_record.s3.bucket.name.unwrap())
        .key(event_record.s3.object.key.unwrap())
        .send()
        .await?;
        let data = resp.body.collect().await;
        let buf = Cursor::new(data?.into_bytes());
        let rdr = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(buf);
        let mut iter = rdr.into_deserialize();
    
        if let Some(result) = iter.next() {
            let record: Embassy = result?;
            println!("{:?}", record);

        } else {
            println!("Error parsing csv");
        }
    };
    // No extra configuration is needed as long as your Lambda has
    // the necessary permissions attached to its role.

    Ok(json!({ "message": "Done parsing csv"}))
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
