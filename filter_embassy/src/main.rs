#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_async;

use slog::Drain;
use std::io::{Cursor};
use std::iter::Iterator;
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::types::AggregatedBytes;
use parquet::file::reader::{FileReader, SerializedFileReader};
use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    // SETTING LOGGER
    let drain = slog_json::Json::new(std::io::stdout())
        .set_pretty(true)
        .add_default_keys()
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = slog::Logger::root(drain, o!("format" => "pretty"));

    // S3 CLIENT
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    // Extract some useful information from the request
    info!(log,"received event");
    // DOWNLOAD CSV
    let resp = s3_client
    .get_object()
    .bucket("clean-embassies")
    .key("embassies.parquet")
    .send()
    .await?;
    let data = resp.body.collect().await?;
    let buf = Cursor::new(data.into_bytes());
    let reader = SerializedFileReader::new(buf).unwrap();

    let parquet_metadata = reader.metadata();
    let mut body_str = "".to_string();
    // & is key
    for row in reader.get_row_iter() {
        body_str.push_str("{row}");
    }
    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(body_str.into())
        .map_err(Box::new)?;
    Ok(resp)
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
