use azure_storage::clients::CloudLocation;
use azure_storage_blobs::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    env_logger::init();

    // this is how you use the emulator.
    let container_client = ContainerClientBuilder::with_location(
        CloudLocation::Emulator {
            address: "127.0.0.1".to_string(),
            port: 10000,
        },
        "emulcont",
    )
    .build();

    // create container
    container_client
        .create()
        .public_access(PublicAccess::None)
        .into_future()
        .await?;

    let res = container_client
        .list_blobs()
        .include_metadata(true)
        .into_stream()
        .next()
        .await
        .expect("stream failed")?;
    println!("{:?}", res);

    Ok(())
}
