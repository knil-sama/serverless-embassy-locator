#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_async;

use slog::Drain;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Serialize, Deserialize};
use serde::de::Deserializer;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use aws_sdk_s3::types::ByteStream;

// can't import from private custom_serde
/// Deserializes `Option<String>`, mapping JSON `null` or the empty string `""` to `None`.
#[cfg(not(feature = "string-null-empty"))]
pub(crate) fn deserialize_lambda_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::deserialize(deserializer)? {
        Some(s) =>
        {
            #[allow(clippy::comparison_to_empty)]
            if s == "" {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
        None => Ok(None),
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventBridgeDetail {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeEvent {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub version: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub detail_type: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub source: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub account: Option<String>,
    pub time: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    pub detail: EventBridgeDetail,

}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(_: LambdaEvent<EventBridgeEvent>) -> Result<Value, Error> {
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
    let url_source_csv = "https://raw.githubusercontent.com/database-of-embassies/database-of-embassies/master/database_of_embassies.csv";
    let response = reqwest::get(url_source_csv).await?;
    let content =  response.text().await?;
    let body = ByteStream::from(content.as_bytes().to_owned());
    // TODO use key for naming instead ? and remove file extensions
    let output_filename = "github_embassies.csv";
    let _resp = s3_client
    .put_object()
    .bucket("raw-embassies")
    .key(output_filename)
    .body(body)
    .send()
    .await?;
    info!(log,"Done writing s3 csv");
    Ok(json!({ "message": "lambda completed"}))
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
