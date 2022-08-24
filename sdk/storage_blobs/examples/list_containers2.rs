use azure_storage::clients::StorageCredentials;
use azure_storage_blobs::prelude::*;
use futures::StreamExt;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct SampleEntity {
    pub something: String,
}

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    // First we retrieve the account name and access key from environment variables.
    let account =
        std::env::var("STORAGE_ACCOUNT").expect("Set env variable STORAGE_ACCOUNT first!");
    let access_key =
        std::env::var("STORAGE_ACCESS_KEY").expect("Set env variable STORAGE_ACCESS_KEY first!");

    let blob_service_client = BlobServiceClientBuilder::new(
        &account,
        StorageCredentials::Key(account.clone(), access_key.clone()),
    )
    .build();

    let response = blob_service_client
        .list_containers()
        .into_stream()
        .next()
        .await
        .expect("stream failed")?;
    println!("response = {:#?}", response);

    let container_client = ContainerClientBuilder::new(
        &account,
        "$logs",
        StorageCredentials::Key(account.clone(), access_key),
    )
    .build();

    let response = container_client
        .list_blobs()
        .into_stream()
        .next()
        .await
        .expect("stream failed")?;
    println!("response = {:#?}", response);

    Ok(())
}
