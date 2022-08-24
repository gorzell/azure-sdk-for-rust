use crate::service::operations::*;
use azure_core::{Context, Pipeline, Request, Response, TimeoutPolicy};
use azure_storage::clients::{
    new_pipeline_from_options, CloudLocation, StorageCredentials, StorageOptions,
};
use url::Url;

#[derive(Clone, Debug)]
pub struct BlobServiceClientBuilder {
    cloud_location: CloudLocation,
    storage_options: StorageOptions,
}

impl BlobServiceClientBuilder {
    #[must_use]
    pub fn new(
        account: impl Into<String>,
        storage_credentials: impl Into<StorageCredentials>,
    ) -> Self {
        let account = account.into();
        let storage_credentials = storage_credentials.into();
        let cloud_location = CloudLocation::Public {
            account,
            storage_credentials,
        };
        Self::with_location(cloud_location)
    }

    #[must_use]
    pub fn with_location(cloud_location: CloudLocation) -> Self {
        Self {
            cloud_location,
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

    pub fn build(self) -> BlobServiceClient {
        // TODO: Errors?
        let storage_credentials = self.cloud_location.storage_credentials();
        let pipeline = new_pipeline_from_options(self.storage_options, storage_credentials.clone());
        let url = self.cloud_location.url("blob").unwrap();
        BlobServiceClient {
            storage_account: self.cloud_location.storage_account().to_string(),
            storage_credentials,
            url,
            pipeline,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlobServiceClient {
    storage_account: String,
    storage_credentials: StorageCredentials,
    url: Url,
    pipeline: Pipeline,
}

impl BlobServiceClient {
    pub fn find_blobs_by_tags(&self, expression: String) -> FindBlobsByTagsBuilder {
        FindBlobsByTagsBuilder::new(self.clone(), expression)
    }

    pub fn list_containers(&self) -> ListContainersBuilder {
        ListContainersBuilder::new(self.clone())
    }

    pub fn account_name(&self) -> &str {
        &self.storage_account
    }

    pub fn storage_credentials(&self) -> &StorageCredentials {
        &self.storage_credentials
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
