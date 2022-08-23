#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_async;

use slog::Drain;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};

use serde::{Serialize, Deserialize};
use serde::de::Deserializer;
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder};
use serde_json::{json, Value};
use std::io::{Cursor};
use std::iter::Iterator;
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::types::AggregatedBytes;
use arrow2::array::*;
use arrow2::io::parquet::write::*;
use arrow2::chunk::Chunk;
use arrow2_convert::ArrowField;
use arrow2_convert::serialize::TryIntoArrow;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ArrowField)]
#[serde(rename_all = "camelCase")]
struct Embassy {
    operator: String,
    #[serde(rename = "operatorQID")]
    operator_qid: String,
    jurisdictions: String,
    #[serde(rename = "jurisdictionQIDs")]
    jurisdiction_qids: String,
    country: String,
    #[serde(rename = "countryQID")]
    country_qid: String,
    city: String,
    #[serde(rename = "cityQID")]
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
    #[serde(rename = "pictureLicenseURL")]
    picture_license_url: String,
    #[serde(rename = "type")]
    role: String,
    #[serde(rename = "typeQID")]
    role_qid: String,
    creation: String,
    #[serde(rename = "QID")]
    qid: String
}

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
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeEventDetailObject {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub key: Option<String>,
    pub size: i64,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub sequencer: Option<String>
}


#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeDetailBucket {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub name: Option<String>
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeDetail {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub version: Option<String>,
    pub bucket: EventBridgeDetailBucket,
    pub object: EventBridgeEventDetailObject,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub requester: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub source_ip_address: Option<String>, // type for ip ?
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub reason: Option<String> // enum ?
}
/// https://docs.aws.amazon.com/AmazonS3/latest/userguide/ev-events.html
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeEvent {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub version: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub id: Option<String>, // uid ?
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub detail_type: Option<String>, // could be an enum ?
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub source: Option<String>,
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub account: Option<String>,
    pub time: DateTime<Utc>,
    #[serde(default)]
    pub resources: Vec<String>,
    pub detail: EventBridgeDetail,
}

fn read_csv(data: AggregatedBytes) -> (Vec<Embassy>, Vec<csv::Error>) {
    let buf = Cursor::new(data.into_bytes());
    let rdr = ReaderBuilder::new()
    .delimiter(b';')
    .flexible(false) // avoid error when row contain ; more
    .from_reader(buf);
    let iter = rdr.into_deserialize();
    let (valid_rows, errors): (Vec<_>, Vec<_>) = iter.partition(Result::is_ok);
    let valid_rows: Vec<Embassy> = valid_rows.into_iter().map(Result::unwrap).collect();
    let errors: Vec<csv::Error> = errors.into_iter().map(Result::unwrap_err).collect();
    (valid_rows, errors)
}
/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<EventBridgeEvent>) -> Result<Value, Error> {
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
    .bucket(event.payload.detail.bucket.name.unwrap())
    .key(event.payload.detail.object.key.unwrap())
    .send()
    .await?;
    let data = resp.body.collect().await?;

    let (valid_rows, errors)= read_csv(data);

    let number_of_valid_records = valid_rows.len().to_string();
    let number_of_errors = errors.len().to_string();
    info!(log,"number of valid rows :{number_of_valid_records}");
    info!(log,"number of errors :{number_of_errors}");
    let first_error = errors.into_iter().next().unwrap();
    // currently "msg": "number of errors :1257",
    info!(log,"first error :{first_error}");
    info!(log,"Done parsing csv");
    // TODO use key for naming instead ? and remove file extensions
    let output_filename = "embassie.parquet";
    //https://github.com/apache/arrow-rs/issues/1927
    // convert vec of record to arrow
    let arrow_array_a: Box<dyn Array> = valid_rows.try_into_arrow().unwrap();
    // can't clone dyn Array so we have to duplicate stuff here, probably wrong but will work
    let arrow_array_b: Box<dyn Array> = valid_rows.try_into_arrow().unwrap();

    // need StructArray to get fields
    let data_type = arrow_array_a.data_type().to_owned();
    // thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', src/main.rs:219:50
    // ignore this shit
    //let validity = Some(arrow_array_a.validity().unwrap_or(None).to_owned());
    let arrays = vec![Arc::from(arrow_array_a)];
    // From fields we got schema
    let struct_array: StructArray = StructArray::from_data(data_type, arrays, None);
    let chunk =Chunk::new(vec![arrow_array_b]);
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Snappy,
        version: Version::V1,
    };
    let schema = arrow2::datatypes::Schema::from(struct_array.fields().to_vec());
    let row_groups = RowGroupIterator::try_new(
        vec![Ok(chunk)].into_iter(),
        &schema,
        options,
        vec![vec![Encoding::Plain], vec![Encoding::Plain]],
    )?;

    // anything implementing `std::io::Write` works
    let file = vec![];

    let mut writer = FileWriter::try_new(file, schema, options)?;

    // Write the file.
    for group in row_groups {
        writer.write(group?)?;
    }
    let _ = writer.end(None)?;

    let body = ByteStream::from(writer.into_inner());
    let _resp = s3_client
    .put_object()
    .bucket("clean-embassies")
    .key(output_filename)
    .body(body)
    .send()
    .await?;
    info!(log,"Done writing s3 parquet");
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
