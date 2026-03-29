use super::*;
use crate::config::{MetricConfig, MetricType as ConfigMetricType, OtlpExporterConfig};
use crate::metrics::{MetricStore, MetricType, MetricValue};
use std::collections::BTreeMap;
use std::time::Duration;

fn sample_gauge() -> MetricValue {
    let mut labels = BTreeMap::new();
    labels.insert("device".to_string(), "plc1".to_string());
    MetricValue {
        name: "temperature".to_string(),
        value: 23.5,
        metric_type: MetricType::Gauge,
        labels,
        description: "Temperature reading".to_string(),
        unit: "celsius".to_string(),
        updated_at: std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    }
}

fn sample_counter() -> MetricValue {
    MetricValue {
        name: "energy_total".to_string(),
        value: 42.0,
        metric_type: MetricType::Counter,
        labels: BTreeMap::new(),
        description: "Total energy".to_string(),
        unit: "kWh".to_string(),
        updated_at: std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    }
}

#[test]
fn build_resource_with_labels() {
    let mut labels = HashMap::new();
    labels.insert("service.name".to_string(), "test".to_string());
    let resource = build_resource(&labels);
    // Resource should be built successfully (no panic)
    drop(resource);
}

#[test]
fn build_resource_empty() {
    let resource = build_resource(&HashMap::new());
    drop(resource);
}

#[test]
fn register_observable_gauge() {
    let resource = build_resource(&HashMap::new());
    let provider = SdkMeterProvider::builder().with_resource(resource).build();
    let meter = provider.meter("test");
    let shared: SharedMetricValues = Arc::new(RwLock::new(vec![sample_gauge()]));
    let mut registered = HashSet::new();

    register_new_instruments(&meter, &[sample_gauge()], &mut registered, &shared);
    assert!(registered.contains("temperature"));
    let _ = provider.shutdown();
}

#[test]
fn register_observable_counter() {
    let resource = build_resource(&HashMap::new());
    let provider = SdkMeterProvider::builder().with_resource(resource).build();
    let meter = provider.meter("test");
    let shared: SharedMetricValues = Arc::new(RwLock::new(vec![sample_counter()]));
    let mut registered = HashSet::new();

    register_new_instruments(&meter, &[sample_counter()], &mut registered, &shared);
    assert!(registered.contains("energy_total"));
    let _ = provider.shutdown();
}

#[test]
fn register_instruments_deduplicates() {
    let resource = build_resource(&HashMap::new());
    let provider = SdkMeterProvider::builder().with_resource(resource).build();
    let meter = provider.meter("test");
    let shared: SharedMetricValues = Arc::new(RwLock::new(vec![sample_gauge()]));
    let mut registered = HashSet::new();

    register_new_instruments(&meter, &[sample_gauge()], &mut registered, &shared);
    register_new_instruments(&meter, &[sample_gauge()], &mut registered, &shared);
    // Still only one registered
    assert_eq!(registered.len(), 1);
    let _ = provider.shutdown();
}

#[test]
fn register_instruments_mixed() {
    let resource = build_resource(&HashMap::new());
    let provider = SdkMeterProvider::builder().with_resource(resource).build();
    let meter = provider.meter("test");
    let shared: SharedMetricValues = Arc::new(RwLock::new(vec![sample_gauge(), sample_counter()]));
    let mut registered = HashSet::new();

    register_new_instruments(
        &meter,
        &[sample_gauge(), sample_counter()],
        &mut registered,
        &shared,
    );
    assert_eq!(registered.len(), 2);
    let _ = provider.shutdown();
}

#[test]
fn register_instruments_empty() {
    let resource = build_resource(&HashMap::new());
    let provider = SdkMeterProvider::builder().with_resource(resource).build();
    let meter = provider.meter("test");
    let shared: SharedMetricValues = Arc::new(RwLock::new(Vec::new()));
    let mut registered = HashSet::new();

    register_new_instruments(&meter, &[], &mut registered, &shared);
    assert!(registered.is_empty());
    let _ = provider.shutdown();
}

#[test]
fn otlp_exporter_new_missing_endpoint() {
    let config = OtlpExporterConfig {
        enabled: true,
        endpoint: None,
        timeout: Duration::from_secs(10),
        interval: Duration::from_secs(10),
        headers: HashMap::new(),
    };
    let result = OtlpMetricExporter::new(config);
    assert!(result.is_err());
}

#[test]
fn otlp_exporter_new_with_endpoint() {
    let config = OtlpExporterConfig {
        enabled: true,
        endpoint: Some("http://localhost:4318".to_string()),
        timeout: Duration::from_secs(10),
        interval: Duration::from_secs(60),
        headers: HashMap::new(),
    };
    let result = OtlpMetricExporter::new(config);
    assert!(result.is_ok());
}

#[tokio::test]
async fn otlp_exporter_export_empty_results() {
    let config = OtlpExporterConfig {
        enabled: true,
        endpoint: Some("http://localhost:4318".to_string()),
        timeout: Duration::from_secs(10),
        interval: Duration::from_secs(60),
        headers: HashMap::new(),
    };
    let mut exporter = OtlpMetricExporter::new(config).unwrap();
    // Export with empty results should succeed (no-op)
    let result = crate::exporter::MetricExporter::export(&mut exporter, &[], &HashMap::new()).await;
    assert!(result.is_ok());
    let _ = crate::exporter::MetricExporter::shutdown(&mut exporter).await;
}

#[test]
fn metric_store_integration() {
    let store = MetricStore::new();
    let global = BTreeMap::from([("env".to_string(), "test".to_string())]);
    let collector_labels = BTreeMap::new();
    store.publish("c1", vec![sample_gauge()], &global, &collector_labels);
    let flat = store.all_metrics_flat();
    assert_eq!(flat.len(), 1);
    // Should contain merged labels
    assert!(flat[0].labels.contains_key("env"));
    assert!(flat[0].labels.contains_key("device"));
    assert!(flat[0].labels.contains_key("collector"));
}
