use crate::*;
use std::{fs, io::Write, path::Path};

pub fn validate(config: &Config) -> Result<()> {
    if config.version != 2 {
        return Err(Error::ConfigurationError(
            "unsupported configuration version".into(),
        ));
    }
    config.care.validate()?;
    Command::Limit {
        stop: config.charge_limit,
        start: config
            .start_threshold
            .enabled
            .then_some(config.start_threshold.value),
    }
    .validate()
}
pub fn decode(input: &str) -> Result<Config> {
    let mut value: toml::Value =
        toml::from_str(input).map_err(|e| Error::ConfigurationError(e.to_string()))?;
    // Version 0 had no version field. Preserve its policy, never silently reset it.
    if value.get("version").is_none()
        || value.get("version").and_then(toml::Value::as_integer) == Some(0)
    {
        let table = value
            .as_table_mut()
            .ok_or_else(|| Error::ConfigurationError("expected TOML table".into()))?;
        table.insert("version".into(), 1.into());
        table.entry("start_threshold").or_insert(
            toml::Value::try_from(StartThreshold {
                enabled: false,
                value: 75,
            })
            .map_err(|e| Error::ConfigurationError(e.to_string()))?,
        );
    }
    if value.get("version").and_then(toml::Value::as_integer) == Some(1)
        && let Some(table) = value.as_table_mut()
    {
        table.insert("version".into(), 2.into());
    }
    let c: Config = value
        .try_into()
        .map_err(|e: toml::de::Error| Error::ConfigurationError(e.to_string()))?;
    validate(&c)?;
    Ok(c)
}
pub fn load(path: &Path) -> Result<Config> {
    match fs::read_to_string(path) {
        Ok(s) => decode(&s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(Error::io("read configuration", e)),
    }
}
pub fn save(path: &Path, config: &Config) -> Result<()> {
    validate(config)?;
    let text =
        toml::to_string_pretty(config).map_err(|e| Error::ConfigurationError(e.to_string()))?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::ConfigurationError("missing parent directory".into()))?;
    fs::create_dir_all(parent).map_err(|e| Error::io("create configuration directory", e))?;
    let tmp = path.with_extension("toml.new");
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(&tmp)
        .map_err(|e| Error::io("create configuration transaction", e))?;
    let result = (|| {
        file.write_all(text.as_bytes())
            .map_err(|e| Error::io("write configuration", e))?;
        file.sync_all()
            .map_err(|e| Error::io("sync configuration", e))?;
        drop(file);
        // std uses rename on Unix and MoveFileExW(REPLACE_EXISTING) on Windows.
        // The temporary file is a sibling, so replacement stays on one filesystem.
        fs::rename(&tmp, path).map_err(|e| Error::io("replace configuration", e))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_and_rejects_bad_policy() {
        for source in [
            "charge_limit = 80",
            "version = 0\ncharge_limit = 80",
            "version = 1\ncharge_limit = 80\n[start_threshold]\nenabled = true\nvalue = 75",
        ] {
            let config = decode(source).unwrap();
            assert_eq!(config.version, 2);
            assert_eq!(config.charge_limit, Some(80));
            assert_eq!(config.care, CareSettings::default());
        }
        assert!(decode("version = 99\ncharge_limit = 80").is_err());
        assert!(decode("charge_limit = 0").is_err());
        assert!(
            decode("charge_limit = 80\n[start_threshold]\nenabled = true\nvalue = 90").is_err()
        );
        assert!(decode("charge_limit = 80\nsecret = 'no'").is_err());
    }
    #[test]
    fn saving_again_replaces_the_previous_policy() {
        let root = std::env::temp_dir().join(format!(
            "batterybridge-config-replace-{}",
            std::process::id()
        ));
        let path = root.join("config.toml");
        let mut policy = Config {
            charge_limit: Some(80),
            ..Config::default()
        };
        save(&path, &policy).unwrap();
        policy.charge_limit = Some(85);
        save(&path, &policy).unwrap();
        assert_eq!(load(&path).unwrap(), policy);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stale_transaction_is_preserved() {
        let root = std::env::temp_dir().join(format!(
            "batterybridge-config-transaction-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("config.toml");
        let tmp = path.with_extension("toml.new");
        fs::write(&tmp, "previous transaction").unwrap();
        assert!(save(&path, &Config::default()).is_err());
        assert_eq!(fs::read_to_string(tmp).unwrap(), "previous transaction");
        fs::remove_dir_all(root).unwrap();
    }
}
