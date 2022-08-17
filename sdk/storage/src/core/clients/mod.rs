use crate::authorization_policy::AuthorizationPolicy;
use crate::prelude::{AccountSasPermissions, AccountSasResource, AccountSasResourceType};
use crate::shared_access_signature::account_sas::AccountSharedAccessSignature;
use azure_core::auth::TokenCredential;
use azure_core::error::{Error, ErrorKind, ResultExt};
use azure_core::headers::{HeaderValue, Headers, CONTENT_LENGTH, MS_DATE, VERSION};
use azure_core::request_options::Timeout;
use azure_core::{date, Body, ClientOptions, Method, Pipeline, Request, TimeoutPolicy};
use std::sync::Arc;
use time::OffsetDateTime;
use url::Url;

/// The well-known account used by Azurite and the legacy Azure Storage Emulator.
/// https://docs.microsoft.com/azure/storage/common/storage-use-azurite#well-known-storage-account-and-key
pub const EMULATOR_ACCOUNT: &str = "devstoreaccount1";

/// The well-known account key used by Azurite and the legacy Azure Storage Emulator.
/// https://docs.microsoft.com/azure/storage/common/storage-use-azurite#well-known-storage-account-and-key
pub const EMULATOR_ACCOUNT_KEY: &str =
    "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==";

const AZURE_VERSION: HeaderValue = HeaderValue::from_static("2019-12-12");

#[derive(Clone)]
pub enum StorageCredentials {
    Key(String, String),
    SASToken(Vec<(String, String)>),
    BearerToken(String),
    TokenCredential(Arc<dyn TokenCredential>),
}

impl std::fmt::Debug for StorageCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            StorageCredentials::Key(_, _) => f
                .debug_struct("StorageCredentials")
                .field("credential", &"Key")
                .finish(),
            StorageCredentials::SASToken(_) => f
                .debug_struct("StorageCredentials")
                .field("credential", &"SASToken")
                .finish(),
            StorageCredentials::BearerToken(_) => f
                .debug_struct("StorageCredentials")
                .field("credential", &"BearerToken")
                .finish(),
            StorageCredentials::TokenCredential(_) => f
                .debug_struct("StorageCredentials")
                .field("credential", &"TokenCredential")
                .finish(),
        }
    }
}

impl StorageCredentials {
    pub fn shared_access_signature(
        &self,
        resource: AccountSasResource,
        resource_type: AccountSasResourceType,
        expiry: OffsetDateTime,
        permissions: AccountSasPermissions,
    ) -> azure_core::Result<AccountSharedAccessSignature> {
        match &self {
            StorageCredentials::Key(account, key) => {
                Ok(AccountSharedAccessSignature::new(account.clone(), key.clone(), resource, resource_type, expiry, permissions))
            }
            _ => Err(Error::message(ErrorKind::Other, "failed shared access signature generation. SAS can be generated only from key and account clients")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageOptions {
    pub options: ClientOptions,
    pub timeout_policy: TimeoutPolicy,
}

impl StorageOptions {
    pub fn new() -> StorageOptions {
        Self::default()
    }

    pub fn set_timeout(&mut self, default_timeout: Timeout) {
        self.timeout_policy = TimeoutPolicy::new(Some(default_timeout))
    }
}
/// The cloud with which you want to interact.
// TODO: Other govt clouds?
#[derive(Debug, Clone)]
pub enum CloudLocation {
    /// Azure public cloud
    Public {
        account: String,
        storage_credentials: StorageCredentials,
    },
    /// Azure China cloud
    China {
        account: String,
        storage_credentials: StorageCredentials,
    },
    /// Use the well-known storage emulator
    Emulator { address: String, port: u16 },
    /// A custom base URL
    Custom {
        uri: Url,
        storage_credentials: StorageCredentials,
    },
}

impl CloudLocation {
    /// the base URL for a given cloud location
    pub fn url(&self, storage_type: impl Into<String>) -> azure_core::Result<Url> {
        let storage_type = storage_type.into();
        match self {
            CloudLocation::Public { account, .. } => Url::parse(
                format!("https://{account}.{storage_type}.core.windows.net").as_str(),
            )
            .with_context(ErrorKind::DataConversion, || {
                format!("failed to parse url: https://{account}.{storage_type}.core.windows.net")
            }),
            CloudLocation::China { account, .. } => Url::parse(
                format!("https://{account}.{storage_type}.core.chinacloudapi.cn").as_str(),
            )
            .with_context(ErrorKind::DataConversion, || {
                format!(
                    "failed to parse url: https://{account}.{storage_type}.core.chinacloudapi.cn"
                )
            }),
            CloudLocation::Custom { uri, .. } => Ok(uri.clone()),
            CloudLocation::Emulator { address, port } => {
                Url::parse(format!("https://{address}:{port}").as_str())
                    .with_context(ErrorKind::DataConversion, || {
                        format!("failed to parse url: https://{address}:{port}")
                    })
            }
        }
    }

    pub fn storage_credentials(&self) -> StorageCredentials {
        match self {
            CloudLocation::Public {
                storage_credentials,
                ..
            } => storage_credentials.clone(),
            CloudLocation::China {
                storage_credentials,
                ..
            } => storage_credentials.clone(),
            CloudLocation::Emulator { .. } => StorageCredentials::Key(
                EMULATOR_ACCOUNT.to_string(),
                EMULATOR_ACCOUNT_KEY.to_string(),
            ),
            CloudLocation::Custom {
                storage_credentials,
                ..
            } => storage_credentials.clone(),
        }
    }

    pub fn storage_account(&self) -> &str {
        match self {
            CloudLocation::Public { account, .. } => account,
            CloudLocation::China { account, .. } => account,
            CloudLocation::Emulator { .. } => EMULATOR_ACCOUNT,
            CloudLocation::Custom { .. } => todo!(),
        }
    }
}

/// Create a Pipeline from StorageOptions
pub fn new_pipeline_from_options(
    options: StorageOptions,
    credentials: StorageCredentials,
) -> Pipeline {
    let auth_policy: Arc<dyn azure_core::Policy> = Arc::new(AuthorizationPolicy::new(credentials));

    // The `AuthorizationPolicy` must be the **last** retry policy.
    // Policies can change the url and/or the headers, and the `AuthorizationPolicy`
    // must be able to inspect them or the resulting token will be invalid.
    let per_retry_policies = vec![
        Arc::new(options.timeout_policy) as Arc<dyn azure_core::Policy>,
        auth_policy,
    ];

    Pipeline::new(
        option_env!("CARGO_PKG_NAME"),
        option_env!("CARGO_PKG_VERSION"),
        options.options,
        Vec::new(),
        per_retry_policies,
    )
}

pub fn finalize_request(
    url: Url,
    method: Method,
    headers: Headers,
    request_body: Option<Body>,
) -> azure_core::Result<Request> {
    let dt = OffsetDateTime::now_utc();
    let time = date::to_rfc1123(&dt);

    let mut request = Request::new(url, method);
    for (k, v) in headers {
        request.insert_header(k, v);
    }

    // let's add content length to avoid "chunking" errors.
    match request_body {
        Some(ref b) => request.insert_header(CONTENT_LENGTH, b.len().to_string()),
        None => request.insert_header(CONTENT_LENGTH, "0"),
    };

    request.insert_header(MS_DATE, time);
    request.insert_header(VERSION, AZURE_VERSION);

    if let Some(request_body) = request_body {
        request.set_body(request_body);
    } else {
        request.set_body(azure_core::EMPTY_BODY);
    };

    Ok(request)
}

pub fn url_with_segments<'a, I>(mut url: url::Url, new_segments: I) -> azure_core::Result<url::Url>
where
    I: IntoIterator<Item = &'a str>,
{
    let original_url = url.clone();
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            let message = format!("failed to parse url path segments from '{original_url}'");
            Error::message(ErrorKind::DataConversion, message)
        })?;
        segments.extend(new_segments);
    }
    Ok(url)
}
