use crate::service::operations::*;
use azure_core::{Context, Request, Response};
use crate::clients::ContainerClient;

pub trait AsBlobServiceClient {
    fn blob_service_client(&self) -> BlobServiceClient;
}

impl AsBlobServiceClient for ContainerClient {
    fn blob_service_client(&self) -> BlobServiceClient {
        BlobServiceClient::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct BlobServiceClient {
    pub(crate) container_client: ContainerClient,
}

impl BlobServiceClient {
    pub(crate) fn new(container_client: ContainerClient) -> Self {
        Self { container_client }
    }

    pub fn find_blobs_by_tags(&self, expression: String) -> FindBlobsByTagsBuilder {
        FindBlobsByTagsBuilder::new(self.clone(), expression)
    }

    pub fn list_containers(&self) -> ListContainersBuilder {
        ListContainersBuilder::new(self.clone())
    }

    pub(crate) async fn send(
        &self,
        context: &mut Context,
        request: &mut Request,
    ) -> azure_core::Result<Response> {
        self.container_client
            .send(context, request)
            .await
    }
}
