use super::*;
use std::str::FromStr;

#[test]
fn test_log_output_from_str() {
    assert_eq!(LogOutput::from_str("stdout").unwrap(), LogOutput::Stdout);
    assert_eq!(LogOutput::from_str("stderr").unwrap(), LogOutput::Stderr);
    assert_eq!(LogOutput::from_str("json").unwrap(), LogOutput::Json);
    assert_eq!(LogOutput::from_str("STDOUT").unwrap(), LogOutput::Stdout);
    assert_eq!(LogOutput::from_str("Stderr").unwrap(), LogOutput::Stderr);
    assert!(LogOutput::from_str("invalid").is_err());
}

#[test]
fn test_parse_level_via_std() {
    assert_eq!(
        "trace".parse::<tracing::Level>().unwrap(),
        tracing::Level::TRACE
    );
    assert_eq!(
        "debug".parse::<tracing::Level>().unwrap(),
        tracing::Level::DEBUG
    );
    assert_eq!(
        "info".parse::<tracing::Level>().unwrap(),
        tracing::Level::INFO
    );
    assert_eq!(
        "warn".parse::<tracing::Level>().unwrap(),
        tracing::Level::WARN
    );
    assert_eq!(
        "error".parse::<tracing::Level>().unwrap(),
        tracing::Level::ERROR
    );
    assert_eq!(
        "INFO".parse::<tracing::Level>().unwrap(),
        tracing::Level::INFO
    );
}

#[test]
fn test_parse_level_invalid() {
    assert!("verbose".parse::<tracing::Level>().is_err());
    assert!("".parse::<tracing::Level>().is_err());
}

#[test]
fn test_default_logging_config() {
    let config = LoggingConfig::default();
    assert_eq!(config.level, "info");
    assert_eq!(config.output, LogOutput::Json);
}

#[test]
fn test_init_logging_invalid_level() {
    let config = LoggingConfig {
        level: "invalid".to_string(),
        output: LogOutput::Stdout,
    };
    assert!(init_logging(&config).is_err());
}

#[test]
fn test_init_logging_stdout() {
    let config = LoggingConfig {
        level: "info".to_string(),
        output: LogOutput::Stdout,
    };
    // May fail if another test already initialized the global subscriber,
    // but should not panic — that's the point of try_init.
    let _ = init_logging(&config);
}

#[test]
fn test_init_logging_stderr() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        output: LogOutput::Stderr,
    };
    let _ = init_logging(&config);
}

#[test]
fn test_init_logging_json() {
    let config = LoggingConfig {
        level: "warn".to_string(),
        output: LogOutput::Json,
    };
    let _ = init_logging(&config);
}

#[test]
fn test_logging_config_deserialize() {
    let yaml = r#"
level: debug
output: json
"#;
    let config: LoggingConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.level, "debug");
    assert_eq!(config.output, LogOutput::Json);
}
