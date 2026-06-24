//! zvariant `Value` ↔ `serde_json::Value` conversion.
//!
//! # Type mapping
//!
//! | D-Bus type | JSON type         |
//! |------------|-------------------|
//! | y u8       | number            |
//! | b bool     | boolean           |
//! | n i16      | number            |
//! | q u16      | number            |
//! | i i32      | number            |
//! | u u32      | number            |
//! | x i64      | number            |
//! | t u64      | number            |
//! | d f64      | number            |
//! | s string   | string            |
//! | o path     | string            |
//! | g sig      | string            |
//! | v variant  | `{"v": <inner>}`  |
//! | a array    | array             |
//! | {kv} dict  | object            |
//! | (…) struct | array             |
//! | h fd       | `{"fd": N}`       |

use serde_json::{json, Value as Json};
use zvariant::{Array, Dict, OwnedValue, Structure, Value};

/// Convert a zvariant `Value` to a JSON value.
pub fn value_to_json(v: &Value<'_>) -> Json {
    match v {
        Value::U8(n)          => json!(n),
        Value::Bool(b)        => json!(b),
        Value::I16(n)         => json!(n),
        Value::U16(n)         => json!(n),
        Value::I32(n)         => json!(n),
        Value::U32(n)         => json!(n),
        Value::I64(n)         => json!(n),
        Value::U64(n)         => json!(n),
        Value::F64(n)         => json!(n),
        Value::Str(s)         => json!(s.as_str()),
        Value::Signature(s)   => json!(s.to_string()),
        Value::ObjectPath(p)  => json!(p.as_str()),
        Value::Value(inner)   => json!({ "v": value_to_json(inner) }),
        Value::Array(arr)     => array_to_json(arr),
        Value::Dict(d)        => dict_to_json(d),
        Value::Structure(s)   => struct_to_json(s),
        #[cfg(unix)]
        Value::Fd(fd) => {
            use std::os::fd::AsRawFd;
            json!({ "fd": fd.as_raw_fd() })
        }
    }
}

/// Convert an `OwnedValue` to JSON.
pub fn owned_to_json(v: &OwnedValue) -> Json {
    // OwnedValue implements From<OwnedValue> for Value<'static>
    // We can borrow the inner Value via TryFrom<&OwnedValue>
    if let Ok(val) = Value::try_from(v) {
        value_to_json(&val)
    } else {
        Json::Null
    }
}

fn array_to_json(arr: &Array<'_>) -> Json {
    let items: Vec<Json> = arr.iter().map(value_to_json).collect();
    Json::Array(items)
}

fn dict_to_json(dict: &Dict<'_, '_>) -> Json {
    let mut map = serde_json::Map::new();
    for (k, v) in dict.iter() {
        let key = match k {
            Value::Str(s)        => s.as_str().to_string(),
            Value::ObjectPath(p) => p.as_str().to_string(),
            Value::Signature(s)  => s.to_string(),
            other                => value_to_json(other).to_string(),
        };
        map.insert(key, value_to_json(v));
    }
    Json::Object(map)
}

fn struct_to_json(s: &Structure<'_>) -> Json {
    let items: Vec<Json> = s.fields().iter().map(value_to_json).collect();
    Json::Array(items)
}

/// Convert a JSON value + optional D-Bus signature hint to an `OwnedValue`.
pub fn json_to_owned_value(json: &Json, sig: Option<&str>) -> anyhow::Result<OwnedValue> {
    let v = json_to_value(json, sig)?;
    Ok(OwnedValue::try_from(v)?)
}

fn json_to_value<'a>(json: &Json, sig: Option<&str>) -> anyhow::Result<Value<'a>> {
    match sig {
        Some("y") => {
            Ok(Value::U8(json.as_u64().ok_or_else(|| anyhow::anyhow!("expected u8"))? as u8))
        }
        Some("b") => {
            Ok(Value::Bool(json.as_bool().ok_or_else(|| anyhow::anyhow!("expected bool"))?))
        }
        Some("n") => {
            Ok(Value::I16(json.as_i64().ok_or_else(|| anyhow::anyhow!("expected i16"))? as i16))
        }
        Some("q") => {
            Ok(Value::U16(json.as_u64().ok_or_else(|| anyhow::anyhow!("expected u16"))? as u16))
        }
        Some("i") => {
            Ok(Value::I32(json.as_i64().ok_or_else(|| anyhow::anyhow!("expected i32"))? as i32))
        }
        Some("u") => {
            Ok(Value::U32(json.as_u64().ok_or_else(|| anyhow::anyhow!("expected u32"))? as u32))
        }
        Some("x") => {
            Ok(Value::I64(json.as_i64().ok_or_else(|| anyhow::anyhow!("expected i64"))?))
        }
        Some("t") => {
            Ok(Value::U64(json.as_u64().ok_or_else(|| anyhow::anyhow!("expected u64"))?))
        }
        Some("d") => {
            Ok(Value::F64(json.as_f64().ok_or_else(|| anyhow::anyhow!("expected f64"))?))
        }
        Some("o") => {
            let s = json.as_str().ok_or_else(|| anyhow::anyhow!("expected object path string"))?;
            // Use OwnedObjectPath to avoid lifetime issues
            let path = zvariant::OwnedObjectPath::try_from(s.to_owned())?;
            Ok(Value::ObjectPath(path.into()))
        }
        Some("g") => {
            let s = json.as_str().ok_or_else(|| anyhow::anyhow!("expected signature string"))?;
            use std::str::FromStr;
            let sig_val = zvariant::Signature::from_str(s)
                .map_err(|e| anyhow::anyhow!("invalid signature '{}': {:?}", s, e))?;
            Ok(Value::Signature(sig_val))
        }
        Some("v") => {
            let inner = json_to_value(json, None)?;
            Ok(Value::Value(Box::new(inner)))
        }
        Some(other) if other.starts_with('a') && other.len() > 1 => {
            let arr = json.as_array()
                .ok_or_else(|| anyhow::anyhow!("expected array for sig '{}'", other))?;
            let elem_sig = &other[1..];
            use std::str::FromStr;
            let sig_val = zvariant::Signature::from_str(other)
                .map_err(|e| anyhow::anyhow!("invalid array sig '{}': {:?}", other, e))?;
            let mut zbus_array = zvariant::Array::new(&sig_val);
            for item in arr {
                let elem = json_to_value(item, Some(elem_sig))?;
                zbus_array.append(elem)?;
            }
            Ok(Value::Array(zbus_array))
        }
        // Default / "s" / unknown: infer from JSON
        _ => {
            if let Some(s) = json.as_str() {
                Ok(Value::Str(zvariant::Str::from(s.to_owned())))
            } else if let Some(b) = json.as_bool() {
                Ok(Value::Bool(b))
            } else if let Some(n) = json.as_i64() {
                // Prefer i32 for small values, i64 for large
                if n >= i32::MIN as i64 && n <= i32::MAX as i64 {
                    Ok(Value::I32(n as i32))
                } else {
                    Ok(Value::I64(n))
                }
            } else if let Some(n) = json.as_u64() {
                Ok(Value::U64(n))
            } else if let Some(n) = json.as_f64() {
                Ok(Value::F64(n))
            } else {
                // Fallback: stringify
                Ok(Value::Str(zvariant::Str::from(json.to_string())))
            }
        }
    }
}

/// Convert a JSON array of call arguments to a body suitable for `call_method`.
///
/// - 0 args → `()`
/// - 1 arg  → single value
/// - N args → `Structure` (tuple)
pub fn json_args_to_body(
    args: &[Json],
    sigs: Option<&[&str]>,
) -> anyhow::Result<OwnedValue> {
    if args.is_empty() {
        return Ok(OwnedValue::try_from(Value::Structure(
            zvariant::StructureBuilder::new().build()?,
        ))?);
    }

    if args.len() == 1 {
        let sig = sigs.and_then(|s| s.first().copied());
        return json_to_owned_value(&args[0], sig);
    }

    // Multiple args → structure
    let mut builder = zvariant::StructureBuilder::new();
    for (i, arg) in args.iter().enumerate() {
        let sig = sigs.and_then(|s| s.get(i).copied());
        builder = builder.add_field(json_to_value(arg, sig)?);
    }
    Ok(OwnedValue::try_from(Value::Structure(builder.build()?))?)
}
