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
use bytes::Bytes;
use std::iter::Iterator;
use std::vec::IntoIter;
use aws_sdk_s3::types::ByteStream;
use aws_sdk_s3::Client;
use arrow2::io::parquet::write::*;
use arrow2::chunk::Chunk;
use arrow2::datatypes::{Field, Schema};
use arrow2::{array::{Utf8Array, Array, Float32Array}};
use futures::executor::block_on;

type RowGroupIteratorType = RowGroupIterator<Box<dyn Array>, IntoIter<Result<Chunk<Box<dyn Array>>, arrow2::error::Error>>>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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


#[derive(Debug, Clone, PartialEq, Eq,Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EventBridgeDetailBucket {
    #[serde(deserialize_with = "deserialize_lambda_string")]
    #[serde(default)]
    pub name: Option<String>
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

fn read_csv(data: Bytes) -> (Vec<Embassy>, Vec<csv::Error>) {
    let buf = Cursor::new(data);
    let rdr = ReaderBuilder::new()
    .delimiter(b';')
    .flexible(true) // avoid error when row contain ; more
    .from_reader(buf);
    let iter = rdr.into_deserialize();
    let (valid_rows, errors): (Vec<_>, Vec<_>) = iter.partition(Result::is_ok);
    let valid_rows: Vec<Embassy> = valid_rows.into_iter().map(Result::unwrap).collect();
    let errors: Vec<csv::Error> = errors.into_iter().map(Result::unwrap_err).collect();
    (valid_rows, errors)
}

fn generate_arrow(valid_rows: Vec<Embassy>) -> (Schema, Chunk<Box<dyn Array>>) {
    let operators: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.operator.as_str())).collect::<Vec<_>>();
    let operator_array = Utf8Array::<i32>::from(operators);
    let field_operator = Field::new("operator", operator_array.data_type().clone(), false);
    
    let countries: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.country.as_str())).collect::<Vec<_>>();
    let country_array = Utf8Array::<i32>::from(countries);    
    let field_country = Field::new("country", country_array.data_type().clone(), false);

    let addresses: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.address.as_str())).collect::<Vec<_>>();
    let address_array = Utf8Array::<i32>::from(addresses);    
    let field_address = Field::new("address",address_array.data_type().clone(), false);

    let websites: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.website.as_str())).collect::<Vec<_>>();
    let website_array = Utf8Array::<i32>::from(websites);    
    let field_website = Field::new("website",website_array.data_type().clone(), false);

    let phones: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.phone.as_str())).collect::<Vec<_>>();
    let phone_array = Utf8Array::<i32>::from(phones);    
    let field_phone = Field::new("phone",phone_array.data_type().clone(), false);

    let emails: Vec<Option<&str>> = valid_rows.iter().map(|embassie| Some(embassie.email.as_str())).collect::<Vec<_>>();
    let email_array = Utf8Array::<i32>::from(emails);    
    let field_email = Field::new("email",email_array.data_type().clone(), false);

    let latitudes: Vec<Option<f32>> = valid_rows.iter().map(|embassie| Some(embassie.latitude)).collect::<Vec<_>>();
    let latitude_array = Float32Array::from(latitudes);    
    let field_latitude = Field::new("latitude",latitude_array.data_type().clone(), false);

    let longitudes: Vec<Option<f32>> = valid_rows.iter().map(|embassie| Some(embassie.longitude)).collect::<Vec<_>>();
    let longitude_array = Float32Array::from(longitudes);    
    let field_longitude = Field::new("longitude",longitude_array.data_type().clone(), false);
    
    let schema = Schema::from(vec![field_operator, field_country, field_address, field_website, field_phone, field_email, field_latitude, field_longitude]);
    let chunk = Chunk::new(vec![operator_array.boxed(), country_array.boxed(), address_array.boxed(), website_array.boxed(), phone_array.boxed(), email_array.boxed(), latitude_array.boxed(), longitude_array.boxed()]);
    (schema, chunk) 
}

fn convert_arrow_to_parquet(schema: &Schema, options: WriteOptions, chunk:Chunk<Box<dyn Array>>) -> RowGroupIteratorType{
    RowGroupIterator::try_new(
        vec![Ok(chunk)].into_iter(),
        schema,
        options,
        vec![vec![Encoding::Plain]; schema.fields.len()],
    ).unwrap()
}

async fn download_from_s3_to_in_memory(s3_client: &Client, bucket: String, key: String) -> Bytes {
        // DOWNLOAD CSV
        let resp = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await;
        resp.unwrap().body.collect().await.unwrap().into_bytes()
}
async fn write_to_s3(s3_client: &Client, bucket: String, key: String, body: ByteStream) {
    let _resp = s3_client
    .put_object()
    .bucket(bucket)
    .key(key)
    .body(body)
    .send()
    .await;
}
fn write_parquet_in_memory(schema: Schema, options: WriteOptions, row_groups: RowGroupIteratorType) -> FileWriter<Vec<u8>> {
        // anything implementing `std::io::Write` works
        let file = vec![];
        let mut writer = FileWriter::try_new(file, schema, options).unwrap();
    
        // Write the file.
        for group in row_groups {
            writer.write(group.unwrap()).unwrap();
        }
        let _ = writer.end(None).unwrap();
        writer
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
    let data = block_on(download_from_s3_to_in_memory(&s3_client, event.payload.detail.bucket.name.unwrap(),event.payload.detail.object.key.unwrap()));

    // replace with arrow read ? https://github.com/jorgecarleitao/arrow2/blob/v0.13.1/examples/csv_read.rs
    // seem less flexible to handle error and specify schema
    let (valid_rows, errors)= read_csv(data);

    // BUNCH OF LOG
    let number_of_valid_records = valid_rows.len().to_string();
    let number_of_errors = errors.len().to_string();
    info!(log,"number of valid rows :{number_of_valid_records}");
    info!(log,"number of errors :{number_of_errors}");
    //let first_error = errors.into_iter().next().unwrap();
    info!(log,"Done parsing csv");
    if number_of_valid_records < number_of_errors {
        panic!("Too much error")
    }
    let (schema, chunk) = generate_arrow(valid_rows);

    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Snappy,
        version: Version::V1,
    };

    let row_groups = convert_arrow_to_parquet(&schema, options, chunk);

    let writer = write_parquet_in_memory(schema, options, row_groups);

    let body = ByteStream::from(writer.into_inner());
    block_on(write_to_s3(&s3_client, "clean-embassies".to_owned(), "embassies.parquet".to_owned(), body));

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_read_csv() {
        let (valid, invalid) = read_csv("operator;operatorQID;jurisdictions;jurisdictionQIDs;country;countryQID;city;cityQID;address;latitude;longitude;phone;email;website;facebook;twitter;youtube;picture;pictureAuthor;pictureLicense;pictureLicenseURL;type;typeQID;creation;QID\n
        Afghanistan;http://www.wikidata.org/entity/Q889;Australia|New Zealand|Fiji;http://www.wikidata.org/entity/Q408|http://www.wikidata.org/entity/Q664|http://www.wikidata.org/entity/Q712;Australia;http://www.wikidata.org/entity/Q408;Canberra;http://www.wikidata.org/entity/Q3114;;-35.32355;149.09222;;;https://www.canberra.mfa.af/;;;;http://commons.wikimedia.org/wiki/Special:FilePath/Afghan%20Embassy%20in%20Canberra.jpg;Kransky;Public domain;;embassy;http://www.wikidata.org/entity/Q3917681;;http://www.wikidata.org/entity/Q106450319\n
        ffrerg".as_bytes().try_into().unwrap());
        assert_eq!(1, valid.len());
        assert_eq!(1, invalid.len());
    }

    #[test]
    fn can_read_empty_csv() {
        let (valid, invalid) = read_csv("".as_bytes().try_into().unwrap());
        assert_eq!(0, valid.len());
        assert_eq!(0, invalid.len());
    }

    #[test]
    fn can_read_csv_with_empty_header() {
        // will just skip first row
        let (valid, invalid) = read_csv("Afghanistan;http://www.wikidata.org/entity/Q889;Australia|New Zealand|Fiji;http://www.wikidata.org/entity/Q408|http://www.wikidata.org/entity/Q664|http://www.wikidata.org/entity/Q712;Australia;http://www.wikidata.org/entity/Q408;Canberra;http://www.wikidata.org/entity/Q3114;;-35.32355;149.09222;;;https://www.canberra.mfa.af/;;;;http://commons.wikimedia.org/wiki/Special:FilePath/Afghan%20Embassy%20in%20Canberra.jpg;Kransky;Public domain;;embassy;http://www.wikidata.org/entity/Q3917681;;http://www.wikidata.org/entity/Q106450319\nAfghanistan;http://www.wikidata.org/entity/Q889;Australia|New Zealand|Fiji;http://www.wikidata.org/entity/Q408|http://www.wikidata.org/entity/Q664|http://www.wikidata.org/entity/Q712;Australia;http://www.wikidata.org/entity/Q408;Canberra;http://www.wikidata.org/entity/Q3114;;-35.32355;149.09222;;;https://www.canberra.mfa.af/;;;;http://commons.wikimedia.org/wiki/Special:FilePath/Afghan%20Embassy%20in%20Canberra.jpg;Kransky;Public domain;;embassy;http://www.wikidata.org/entity/Q3917681;;http://www.wikidata.org/entity/Q106450319\n".as_bytes().try_into().unwrap());
        assert_eq!(0, valid.len());
        assert_eq!(0, invalid.len());
    }
}
