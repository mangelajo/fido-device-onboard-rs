use glob::glob;
use serde::Deserialize;
use std::path::Path;
use std::{env, path::PathBuf};

use anyhow::{bail, Context, Result};

use serde_cbor::Value as CborValue;
use serde_yaml::Value;

#[derive(Debug)]
pub struct AbsolutePathBuf(PathBuf);

impl<'de> Deserialize<'de> for AbsolutePathBuf {
    fn deserialize<D>(deserializer: D) -> Result<AbsolutePathBuf, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Err(serde::de::Error::custom("path is empty".to_string()));
        }
        let path = PathBuf::from(&s);
        if !path.is_absolute() {
            return Err(serde::de::Error::custom(format!(
                "path {} is not absolute",
                s
            )));
        }
        Ok(AbsolutePathBuf(path))
    }
}

impl std::fmt::Display for AbsolutePathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl AsRef<Path> for AbsolutePathBuf {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

// TODO(runcom): find a better home for this as it's shared between
// owner-onboarding-server and manufacturing-server...
#[non_exhaustive]
pub enum OwnershipVoucherStoreMetadataKey {
    To2Performed,
    To0AcceptOwnerWaitSeconds,
}

impl fdo_store::MetadataLocalKey for OwnershipVoucherStoreMetadataKey {
    fn to_key(&self) -> &'static str {
        match self {
            OwnershipVoucherStoreMetadataKey::To2Performed => "fdo.to2_performed",
            OwnershipVoucherStoreMetadataKey::To0AcceptOwnerWaitSeconds => {
                "fdo.to0_accept_owner_wait_seconds"
            }
        }
    }
}

pub fn settings_for(component: &str) -> Result<config::Config> {
    Ok(config::Config::default()
        .merge(
            config::File::from(Path::new(&format!("/usr/share/fdo/{}.yml", component)))
                .required(false),
        )
        .context("Loading configuration file from /usr/share/fdo")?
        .merge(
            config::File::from(Path::new(
                &conf_dir_from_env(&format_conf_env(component))
                    .unwrap_or_else(|| format!("/etc/fdo/{}.yml", component)),
            ))
            .required(false),
        )
        .context("Loading configuration file from /etc/fdo")?
        .merge(
            glob(
                &conf_dir_from_env(&format_conf_dir_env(component))
                    .unwrap_or_else(|| format!("/etc/fdo/{}.conf.d/*.yml", component)),
            )?
            .map(|path| config::File::from(path.unwrap()))
            .collect::<Vec<_>>(),
        )
        .context("Loading configuration files from conf.d")?
        .clone())
}

pub fn format_conf_env(component: &str) -> String {
    format!("{}_CONF", component_env_prefix(component))
}

pub fn format_conf_dir_env(component: &str) -> String {
    format!("{}_CONF_DIR", component_env_prefix(component))
}

fn component_env_prefix(component: &str) -> String {
    component.to_string().replace("-", "_").to_uppercase()
}

fn conf_dir_from_env(key: &str) -> Option<String> {
    match env::var_os(key) {
        None => None,
        Some(v) => match v.into_string() {
            Ok(s) => Some(s),
            Err(_) => None,
        },
    }
}

pub fn yaml_to_cbor(val: &Value) -> Result<CborValue> {
    Ok(match val {
        Value::Null => CborValue::Null,
        Value::Bool(b) => CborValue::Bool(*b),
        Value::Number(nr) => {
            if let Some(nr) = nr.as_u64() {
                CborValue::Integer(nr as i128)
            } else if let Some(nr) = nr.as_i64() {
                CborValue::Integer(nr as i128)
            } else if let Some(nr) = nr.as_f64() {
                CborValue::Float(nr)
            } else {
                bail!("Invalid number encountered");
            }
        }
        Value::String(str) => CborValue::Text(str.clone()),
        Value::Sequence(seq) => CborValue::Array(
            seq.iter()
                .map(yaml_to_cbor)
                .collect::<Result<Vec<CborValue>>>()?,
        ),
        Value::Mapping(map) => CborValue::Map(
            map.iter()
                .map(|(key, val)| (yaml_to_cbor(key).unwrap(), yaml_to_cbor(val).unwrap()))
                .collect(),
        ),
    })
}
