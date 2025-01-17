use mongodb::Client;
use mongodb::options::{ClientOptions, ServerApi, ServerApiVersion};

pub const USER_API_KEY: &str = env!("USER_API_KEY");
pub const MONGO_URL: &str = env!("MONGO_URL");
const MONGO_CON: tokio::sync::OnceCell<mongodb::Database> = tokio::sync::OnceCell::const_new();
pub async fn mongo() -> mongodb::Database {
    MONGO_CON
        .get_or_init(|| async {
            // Replace the placeholder with your Atlas connection string
            let mut client_options = ClientOptions::parse(MONGO_URL).await.unwrap();
            // Set the server_api field of the client_options object to Stable API version 1
            let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
            client_options.server_api = Some(server_api);
            // Create a new client and connect to the server
            let client = Client::with_options(client_options).unwrap();
            client.database("wt")
        })
        .await
        .clone()
}