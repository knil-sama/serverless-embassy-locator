# serverless-embassy-locator

goal of this project is to have a simple use case for new aws features: [s3 Object Lambda](https://aws.amazon.com/blogs/aws/introducing-amazon-s3-object-lambda-use-your-code-to-process-data-as-it-is-being-retrieved-from-s3/)

1. Fetch individual files for each countries locating their embassies
2. Push file on s3 (private)
3. Create s3 Object Lambda function to filter closest ambassy using nationality and coordinate

s3 Object feature is only available on console, cli and aws sdk.

So we will use aws cli for setting thing up

```
init.sh
fetch.sh
sync_to_s3.sh
create_access_point.sh
```
