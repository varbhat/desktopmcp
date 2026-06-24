//! AT-SPI Value interface.
//!
//! The Value interface provides access to numeric widgets such as sliders,
//! spin buttons, progress bars, and scroll bars.

use anyhow::{Context, Result};
use zbus::Connection;
use zbus::fdo::PropertiesProxy;

use super::types::{ElementId, ValueInfo};

/// Get the current, minimum, maximum, and increment values of a Value widget.
pub async fn get_value(conn: &Connection, id: &ElementId) -> Result<ValueInfo> {
    let (bus, path) = id.parts()?;

    let props = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    let iface: zbus::names::InterfaceName<'_> = "org.a11y.atspi.Value".try_into()?;

    let current_val = props.get(iface.clone(), "CurrentValue").await.context("Value.CurrentValue")?;
    let min_val = props.get(iface.clone(), "MinimumValue").await.context("Value.MinimumValue")?;
    let max_val = props.get(iface.clone(), "MaximumValue").await.context("Value.MaximumValue")?;
    let step_val = props.get(iface.clone(), "MinimumIncrement").await.context("Value.MinimumIncrement")?;

    Ok(ValueInfo {
        current: extract_f64(&current_val)?,
        minimum: extract_f64(&min_val)?,
        maximum: extract_f64(&max_val)?,
        minimum_increment: extract_f64(&step_val).unwrap_or(0.0),
    })
}

/// Set the current value of a Value widget.
pub async fn set_value(conn: &Connection, id: &ElementId, value: f64) -> Result<()> {
    let (bus, path) = id.parts()?;

    use zvariant::Value;
    let props = PropertiesProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .build()
        .await?;

    props
        .set(
            "org.a11y.atspi.Value".try_into()?,
            "CurrentValue",
            Value::F64(value),
        )
        .await
        .context("Set Value.CurrentValue")?;

    Ok(())
}

fn extract_f64(val: &zvariant::OwnedValue) -> Result<f64> {
    use zvariant::Value;
    let v = Value::try_from(val).context("extract_f64")?;
    match v {
        Value::F64(n) => Ok(n),
        Value::Value(inner) => match *inner {
            Value::F64(n) => Ok(n),
            Value::I32(n) => Ok(n as f64),
            Value::U32(n) => Ok(n as f64),
            other => anyhow::bail!("Expected f64, got {:?}", other),
        },
        Value::I32(n) => Ok(n as f64),
        Value::U32(n) => Ok(n as f64),
        other => anyhow::bail!("Expected f64, got {:?}", other),
    }
}
