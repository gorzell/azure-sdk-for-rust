use crate::{clients::*, container::operations::*, prelude::PublicAccess};
use azure_core::{
    error::{Error, ErrorKind},
    prelude::*,
    Pipeline, Request, Response, TimeoutPolicy, Url,
};
use azure_storage::clients::{new_pipeline_from_options, CloudLocation, StorageOptions};
use azure_storage::{
    core::clients::StorageCredentials,
    prelude::BlobSasPermissions,
    shared_access_signature::{
        service_sas::{BlobSharedAccessSignature, BlobSignedResource},
        SasToken,
    },
};
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct ContainerClientBuilder {
    cloud_location: CloudLocation,
    container: String,
    storage_options: StorageOptions,
}

impl ContainerClientBuilder {
    #[must_use]
    pub fn new(
        account: impl Into<String>,
        container: impl Into<String>,
        storage_credentials: impl Into<StorageCredentials>,
    ) -> Self {
        let account = account.into();
        let container = container.into();
        let storage_credentials = storage_credentials.into();
        let cloud_location = CloudLocation::Public {
            account,
            storage_credentials,
        };
        Self::with_location(cloud_location, container)
    }

    #[must_use]
    pub fn with_location(cloud_location: CloudLocation, container: impl Into<String>) -> Self {
        let container = container.into();
        Self {
            cloud_location,
            container,
            storage_options: StorageOptions::default(),
        }
    }

    #[must_use]
    pub fn cloud_location(mut self, cloud_location: CloudLocation) -> Self {
        self.cloud_location = cloud_location;
        self
    }

    #[must_use]
    pub fn retry(mut self, retry: impl Into<azure_core::RetryOptions>) -> Self {
        self.storage_options.options = self.storage_options.options.retry(retry);
        self
    }

    #[must_use]
    pub fn transport(mut self, transport: impl Into<azure_core::TransportOptions>) -> Self {
        self.storage_options.options = self.storage_options.options.transport(transport);
        self
    }

    #[must_use]
    pub fn timeout(mut self, timeout: impl Into<TimeoutPolicy>) -> Self {
        let timeout = timeout.into();
        self.storage_options.timeout_policy = timeout;
        self
    }

    pub fn build(self) -> ContainerClient {
        // TODO: Errors?
        let storage_credentials = self.cloud_location.storage_credentials();
        let pipeline = new_pipeline_from_options(self.storage_options, storage_credentials.clone());
        let url = self
            .cloud_location
            .url("blob")
            .unwrap()
            .join(&self.container)
            .unwrap();
        ContainerClient {
            storage_account: self.cloud_location.storage_account().to_string(),
            storage_credentials,
            container_name: self.container,
            url,
            pipeline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContainerClient {
    storage_account: String,
    storage_credentials: StorageCredentials,
    container_name: String,
    url: Url,
    pipeline: Pipeline,
}

impl ContainerClient {
    pub fn create(&self) -> CreateBuilder {
        CreateBuilder::new(self.clone())
    }

    pub fn delete(&self) -> DeleteBuilder {
        DeleteBuilder::new(self.clone())
    }

    pub fn get_acl(&self) -> GetACLBuilder {
        GetACLBuilder::new(self.clone())
    }

    pub fn set_acl(&self, public_access: PublicAccess) -> SetACLBuilder {
        SetACLBuilder::new(self.clone(), public_access)
    }

    pub fn get_properties(&self) -> GetPropertiesBuilder {
        GetPropertiesBuilder::new(self.clone())
    }

    pub fn list_blobs(&self) -> ListBlobsBuilder {
        ListBlobsBuilder::new(self.clone())
    }

    pub fn acquire_lease<LD: Into<LeaseDuration>>(
        &self,
        lease_duration: LD,
    ) -> AcquireLeaseBuilder {
        AcquireLeaseBuilder::new(self.clone(), lease_duration.into())
    }

    pub fn break_lease(&self) -> BreakLeaseBuilder {
        BreakLeaseBuilder::new(self.clone())
    }

    pub fn container_lease_client(&self, lease_id: LeaseId) -> ContainerLeaseClient {
        ContainerLeaseClient::new(self.clone(), lease_id)
    }

    pub fn blob_client<BN: Into<String>>(&self, blob_name: BN) -> BlobClient {
        BlobClient::new(self.clone(), blob_name.into())
    }

    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    pub fn account_name(&self) -> &str {
        &self.storage_account
    }

    pub fn storage_credentials(&self) -> &StorageCredentials {
        &self.storage_credentials
    }

    pub fn shared_access_signature(
        &self,
        permissions: BlobSasPermissions,
        expiry: OffsetDateTime,
    ) -> azure_core::Result<BlobSharedAccessSignature> {
        let canonicalized_resource =
            format!("/blob/{}/{}", &self.storage_account, self.container_name(),);

        match &self.storage_credentials {
            StorageCredentials::Key(_, key) => Ok(
                BlobSharedAccessSignature::new(key.to_string(), canonicalized_resource, permissions, expiry, BlobSignedResource::Container),
            ),
            _ => Err(Error::message(ErrorKind::Credential,
                "Shared access signature generation - SAS can be generated only from key and account clients",
            )),
        }
    }

    pub fn generate_signed_container_url<T>(&self, signature: &T) -> Url
    where
        T: SasToken,
    {
        let mut url = self.url();
        url.set_query(Some(&signature.token()));
        url
    }

    pub fn url(&self) -> Url {
        self.url.clone()
    }

    pub(crate) async fn send(
        &self,
        context: &mut Context,
        request: &mut Request,
    ) -> azure_core::Result<Response> {
        self.pipeline.send(context, request).await
    }
}

#[cfg(test)]
#[cfg(feature = "test_integration")]
mod integration_tests {
    use super::*;
    use crate::{blob::clients::AsBlobClient, core::prelude::*};

    fn get_emulator_client(container_name: &str) -> ContainerClient {
        let storage_account = StorageClient::new_emulator_default();

        storage_account.container_client(container_name)
    }

    #[tokio::test]
    async fn test_create_delete() {
        let container_name = uuid::Uuid::new_v4().to_string();
        let container_client = get_emulator_client(&container_name);

        container_client
            .create()
            .into_future()
            .await
            .expect("create container should succeed");
        container_client
            .delete()
            .into_future()
            .await
            .expect("delete container should succeed");
    }

    #[tokio::test]
    async fn test_list_blobs() {
        let container_name = uuid::Uuid::new_v4().to_string();
        let container_client = get_emulator_client(&container_name);

        container_client
            .create()
            .into_future()
            .await
            .expect("create container should succeed");

        let md5 = md5::compute("world");
        container_client
            .blob_client("hello.txt")
            .put_block_blob("world")
            .into_future()
            .await
            .expect("put block blob should succeed");
        let list = container_client
            .list_blobs()
            .execute()
            .await
            .expect("list blobs should succeed");
        assert_eq!(list.blobs.blobs.len(), 1);
        assert_eq!(list.blobs.blobs[0].name, "hello.txt");
        assert_eq!(
            list.blobs.blobs[0]
                .properties
                .content_md5
                .as_ref()
                .expect("has content_md5")
                .as_slice(),
            &md5.0
        );

        container_client
            .delete()
            .into_future()
            .await
            .expect("delete container should succeed");
    }
}
