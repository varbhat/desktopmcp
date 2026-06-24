use anyhow::Result;
use ashpd::desktop::network_monitor::NetworkMonitor;

pub struct NetworkPortal;

impl NetworkPortal {
    /// Get full network status.
    pub async fn status() -> Result<NetworkStatus> {
        let proxy = NetworkMonitor::new().await?;
        let available = proxy.is_available().await?;
        let metered = proxy.is_metered().await?;
        let connectivity = proxy.connectivity().await?;
        Ok(NetworkStatus {
            available,
            metered,
            connectivity: format!("{:?}", connectivity),
        })
    }

    /// Check if a hostname:port is reachable.
    pub async fn can_reach(hostname: &str, port: u32) -> Result<bool> {
        let proxy = NetworkMonitor::new().await?;
        let reachable = proxy.can_reach(hostname, port).await?;
        Ok(reachable)
    }
}

pub struct NetworkStatus {
    pub available: bool,
    pub metered: bool,
    pub connectivity: String,
}
