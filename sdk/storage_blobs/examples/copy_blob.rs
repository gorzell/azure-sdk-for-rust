use azure_core::date;
use azure_storage::clients::StorageCredentials;
use azure_storage::prelude::{
    AccountSasPermissions, AccountSasResource, AccountSasResourceType, SasProtocol, SasToken,
};
use azure_storage_blobs::prelude::*;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> azure_core::Result<()> {
    // First we retrieve the account name and access key from environment variables.
    let source_account = std::env::var("SOURCE_STORAGE_ACCOUNT")
        .expect("Set env variable SOURCE_STORAGE_ACCOUNT first!");
    let source_access_key = std::env::var("SOURCE_STORAGE_ACCESS_KEY")
        .expect("Set env variable SOURCE_STORAGE_ACCESS_KEY first!");
    let destination_account = std::env::var("DESTINATION_STORAGE_ACCOUNT")
        .expect("Set env variable DESTINATION_STORAGE_ACCOUNT first!");
    let destination_access_key = std::env::var("DESTINATION_STORAGE_ACCESS_KEY")
        .expect("Set env variable DESTINATION_STORAGE_ACCESS_KEY first!");

    let source_container_name = std::env::args()
        .nth(1)
        .expect("please specify source container name as first command line parameter");
    let source_blob_name = std::env::args()
        .nth(2)
        .expect("please specify source blob name as second command line parameter");

    let destination_container_name = std::env::args()
        .nth(3)
        .expect("please specify destination container name as third command line parameter");
    let destination_blob_name = std::env::args()
        .nth(4)
        .expect("please specify destination blob name as fourth command line parameter");

    let source_blob = ContainerClientBuilder::new(
        &source_account,
        &source_container_name,
        StorageCredentials::Key(source_account.clone(), source_access_key),
    )
    .build()
    .blob_client(&source_blob_name);

    let destination_blob = ContainerClientBuilder::new(
        &destination_account,
        &destination_container_name,
        StorageCredentials::Key(destination_account.clone(), destination_access_key),
    )
    .build()
    .blob_client(&destination_blob_name);

    // let's get a SAS key for the source
    let sas_url = {
        let now = OffsetDateTime::now_utc();
        let later = now + date::duration_from_hours(1);
        let sas = source_blob
            .container_client()
            .storage_credentials()
            .shared_access_signature(
                AccountSasResource::Blob,
                AccountSasResourceType::Object,
                later,
                AccountSasPermissions {
                    read: true,
                    ..Default::default()
                },
            )?
            .start(now)
            .protocol(SasProtocol::HttpHttps);
        println!("token: '{}'", sas.token());

        source_blob.generate_signed_blob_url(&sas)?
    };
    println!("read only SAS url: '{}'", sas_url);

    let response = destination_blob.copy(sas_url).into_future().await?;
    println!("copy response == {:#?}", response);

    Ok(())
}
