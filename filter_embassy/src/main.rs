#[macro_use]
extern crate slog;
extern crate slog_json;
extern crate slog_async;
use slog::Drain;
use std::iter::Iterator;
use parquet::file::reader::{FileReader, SerializedFileReader};
use lambda_http::{run, service_fn, Body, Error, Request, Response};

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(_event: Request) -> Result<Response<Body>, Error> {
    // can't use http localization from http api
    // https://stackoverflow.com/questions/64318725/geolocation-service-with-aws-api-gateway-and-lambda
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
    // DOWNLOAD PARQUET
    let resp = s3_client
    .get_object()
    .bucket("clean-embassies")
    .key("embassies.parquet")
    .send()
    .await?;
    let data = resp.body.collect().await?;
    let columns_to_keep = vec!("operator", "country", "website","phone","email");

    let reader = SerializedFileReader::new(data.into_bytes()).unwrap();
    let mut body_str = "<input type=\"text\" id=\"nationality\" onkeyup=\"filterByNationality()\" placeholder=\"Search your embassy by nationality\">".to_string();
    // & is key
    body_str.push_str("<table id=\"embassies\">");
    //thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: General("Root schema does not contain projection")', src/main.rs:62:75 
    for (row_number, row) in reader.get_row_iter(None).unwrap().enumerate() {
        // Set header
        if row_number == 0 {
            body_str.push_str("<tr class=\"header\">");  
            for (name, _) in row.get_column_iter() {
              if columns_to_keep.contains(&name.as_str()){
                body_str.push_str(&format!("<th>{}</th>", name));
              }
            }
            body_str.push_str("<tr>");  
        }
        body_str.push_str("<tr>");  
        for (name, value) in row.get_column_iter() {
          if columns_to_keep.contains(&name.as_str()){
            body_str.push_str(&format!("<td>{}</td>", value.to_string().replace('"',"")));
          }
        }
        body_str.push_str("<tr>"); 
    }
    body_str.push_str("</table>");
    body_str.push_str("<script>
    const filterByNationality = () => {
      const trs = document.querySelectorAll('#embassies tr:not(.header)')
      const filter = document.querySelector('#nationality').value
      const regex = new RegExp(filter, 'i')
      const isFoundInTds = td => regex.test(td.innerHTML)
      const isFound = childrenArr => childrenArr.some(isFoundInTds)
      const setTrStyleDisplay = ({ style, children }) => {
        style.display = isFound([
          ...children // <-- All columns
        ]) ? '' : 'none' 
      }
      
      trs.forEach(setTrStyleDisplay)
    }
</script>");
    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(body_str.into())
        .map_err(Box::new)?;
    info!(log,"Done sending http data");
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
