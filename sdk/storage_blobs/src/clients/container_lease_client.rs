use crate::{container::operations::*, prelude::*};
use azure_core::{prelude::*, Context, Request, Response};

#[derive(Debug, Clone)]
pub struct ContainerLeaseClient {
    container_client: ContainerClient,
    lease_id: LeaseId,
}

impl ContainerLeaseClient {
    pub(crate) fn new(container_client: ContainerClient, lease_id: LeaseId) -> Self {
        Self {
            container_client,
            lease_id,
        }
    }

    pub fn release(&self) -> ReleaseLeaseBuilder {
        ReleaseLeaseBuilder::new(self.clone())
    }

    pub fn renew(&self) -> RenewLeaseBuilder {
        RenewLeaseBuilder::new(self.clone())
    }

    pub fn lease_id(&self) -> LeaseId {
        self.lease_id
    }

    pub fn container_client(&self) -> &ContainerClient {
        &self.container_client
    }

    pub(crate) fn url(&self) -> url::Url {
        self.container_client.url()
    }

    pub(crate) async fn send(
        &self,
        context: &mut Context,
        request: &mut Request,
    ) -> azure_core::Result<Response> {
        self.container_client.send(context, request).await
    }
}
