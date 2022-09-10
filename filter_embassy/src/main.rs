#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_async;
extern crate fstrings;
use parquet::schema::types::Type;
use slog::Drain;
use std::iter::Iterator;
use parquet::file::reader::{FileReader, SerializedFileReader};
use lambda_http::{run, service_fn, Body, Error, Request, Response};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    // SETTING LOGGER
    let drain = slog_json::Json::new(std::io::stdout())
        .set_pretty(true)
        .add_default_keys()
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = slog::Logger::root(drain, o!("format" => "pretty"));
    // TODO ADD LOGIC TO EXTRACT COUNTRY FROM EVENT
    // S3 CLIENT
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    // Extract some useful information from the request
    info!(log,"received event");
    let event_str = format!("{_event:?}");
    info!(log,"{event_str}");
    // DOWNLOAD PARQUET
    let resp = s3_client
    .get_object()
    .bucket("clean-embassies")
    .key("embassies.parquet")
    .send()
    .await?;
    let data = resp.body.collect().await?;

    // CREATE SCHEMA PROJECTION
    //let parquet_projection = ;
    let reader = SerializedFileReader::new(data.into_bytes()).unwrap();

    let schema: &parquet::schema::types::Type = reader.metadata().file_metadata().schema();
    let requested_fields = vec!("operator", "country", "website", "phone", "email");
    let mut selected_fields = schema.get_fields().to_vec();
    if requested_fields.len()>0{
	    selected_fields.retain(|f|  
		  requested_fields.contains(&f.name()));
    }			
    let schema_projection = Type::group_type_builder("schema")
    .with_fields(&mut selected_fields)
    .build()
    .unwrap();
    let mut body_str = "<input type=\"text\" id=\"nationality\" onkeyup=\"filterByNationality()\" placeholder=\"Search your embassy by nationality\">".to_string();
    // & is key
    body_str.push_str("<table id=\"embassies\">");
    //thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: General("Root schema does not contain projection")', src/main.rs:62:75 
    for (row_number, row) in reader.get_row_iter(Some(schema_projection)).unwrap().enumerate() {
        // Set header
        if row_number == 0 {
            body_str.push_str("<tr>");  
            for (name, _) in row.get_column_iter() {
                body_str.push_str(&format!("<th>{}</th>", name));
            }
            body_str.push_str("<tr>");  
        }
        body_str.push_str("<tr>");  
        for (_, value) in row.get_column_iter() {
            body_str.push_str(&format!("<td>{}</td>", value.to_string().replace("\"", "")));
        }
        body_str.push_str("<tr>"); 
    }
    body_str.push_str("</table>");
    body_str.push_str("<script>
function filterByNationality() {
  // Declare variables
  var input, filter, table, tr, td, i, txtValue;
  var operatorColumnId = 0;
  input = document.getElementById(\"nationality\");
  filter = input.value.toUpperCase();
  table = document.getElementById(\"embassies\");
  tr = table.getElementsByTagName(\"tr\");
  // Loop through all table rows, and hide those who don't match the search query
  for (i = 0; i < tr.length; i++) {
    td = tr[i].getElementsByTagName(\"td\")[operatorColumnId];
    if (td) {
      txtValue = td.textContent || td.innerText;
      if (txtValue.toUpperCase().indexOf(filter) > -1) {
        tr[i].style.display = \"\";
      } else {
        tr[i].style.display = \"none\";
      }
    }
  }
}
</script>");
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
