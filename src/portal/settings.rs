use anyhow::Result;
use ashpd::desktop::settings::Settings;
use std::collections::HashMap;

pub struct SettingsPortal;

impl SettingsPortal {
    /// Read a specific desktop setting as a string representation.
    pub async fn read(namespace: &str, key: &str) -> Result<String> {
        let proxy = Settings::new().await?;
        if let Ok(value) = proxy.read::<String>(namespace, key).await {
            return Ok(value);
        }
        if let Ok(value) = proxy.read::<u32>(namespace, key).await {
            return Ok(value.to_string());
        }
        if let Ok(value) = proxy.read::<bool>(namespace, key).await {
            return Ok(value.to_string());
        }
        if let Ok(value) = proxy.read::<f64>(namespace, key).await {
            return Ok(value.to_string());
        }
        anyhow::bail!("Could not read setting {}.{}", namespace, key)
    }

    /// Read all settings for a list of namespaces.
    ///
    /// Returns a map of `{ namespace: { key: value_string } }`.
    pub async fn read_all(namespaces: &[&str]) -> Result<HashMap<String, HashMap<String, String>>> {
        let proxy = Settings::new().await?;
        let raw = proxy.read_all(namespaces).await?;
        let mut result: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (ns, kvs) in raw {
            let inner: HashMap<String, String> = kvs
                .into_iter()
                .map(|(k, v)| (k, format!("{v:?}")))
                .collect();
            result.insert(ns, inner);
        }
        Ok(result)
    }

    /// Read the color scheme preference (0=default, 1=dark, 2=light).
    pub async fn color_scheme() -> Result<u32> {
        let proxy = Settings::new().await?;
        let value = proxy
            .read::<u32>("org.freedesktop.appearance", "color-scheme")
            .await?;
        Ok(value)
    }

    /// Read the accent color (r, g, b as floats 0.0-1.0).
    pub async fn accent_color() -> Result<(f64, f64, f64)> {
        let proxy = Settings::new().await?;
        let value = proxy
            .read::<(f64, f64, f64)>("org.freedesktop.appearance", "accent-color")
            .await?;
        Ok(value)
    }
}
