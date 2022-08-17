use azure_storage::headers::CommonStorageResponseHeaders;
use azure_core::headers::{account_kind_from_headers, sku_name_from_headers, Headers};
use crate::clients::ContainerClient;

operation! {
    GetAccountInformation,
    client: ContainerClient,
}

impl GetAccountInformationBuilder {
    pub fn into_future(mut self) -> GetAccountInformation {
        Box::pin(async move {
            let mut request = self.client.blob_storage_request(azure_core::Method::Get)?;

            for (k, v) in [("restype", "account"), ("comp", "properties")].iter() {
                request.url_mut().query_pairs_mut().append_pair(k, v);
            }

            let response = self
                .client
                .send(&mut self.context, &mut request)
                .await?;

            GetAccountInformationResponse::try_from(response.headers())
        })
    }
}

#[derive(Debug, Clone)]
pub struct GetAccountInformationResponse {
    pub common: CommonStorageResponseHeaders,
    pub sku_name: String,
    pub account_kind: String,
}

impl GetAccountInformationResponse {
    pub fn try_from(headers: &Headers) -> azure_core::Result<GetAccountInformationResponse> {
        let common = CommonStorageResponseHeaders::try_from(headers)?;
        let sku_name = sku_name_from_headers(headers)?;
        let account_kind = account_kind_from_headers(headers)?;

        Ok(GetAccountInformationResponse {
            common,
            sku_name,
            account_kind,
        })
    }
}
