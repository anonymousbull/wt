use lambda_http::{run, tracing, Error};
use std::env::set_var;


#[tokio::main]
async fn main() -> Result<(), Error> {
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");
    tracing::init_default_subscriber();

    let router = users_api::server::start().await;
    run(router).await
}
