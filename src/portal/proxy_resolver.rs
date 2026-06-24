//! Proxy Resolver portal — resolve proxy settings for a URI.

use anyhow::Result;
use ashpd::desktop::proxy_resolver::ProxyResolver;

pub struct ProxyResolverPortal;

impl ProxyResolverPortal {
    /// Return the list of proxy URIs to use for `uri`.
    ///
    /// E.g. `["direct://"]` or `["http://proxy.example.com:8080/"]`.
    pub async fn lookup(uri: &str) -> Result<Vec<String>> {
        let proxy = ProxyResolver::new().await?;
        let parsed = ashpd::Uri::parse(uri)?;
        let proxies = proxy.lookup(&parsed).await?;
        Ok(proxies.iter().map(|u| u.to_string()).collect())
    }
}
