//! E2E / integration tests for the I3C pipeline.
//!
//! Since there is no real I3C hardware in CI, these tests use mock devices
//! to exercise the full path: config parsing → I3C client → read_metric →
//! decode → validate values.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use bus_exporter::config::{ByteOrder, Config, DataType, Metric, MetricType};
use bus_exporter::i3c::{AddressMode, I3cClient, I3cDevice};

// ── Mock Device ─────────────────────────────────────────────────────

/// Mock I3C device with configurable per-call responses.
struct MockI3cDevice {
    responses: Mutex<Vec<Result<Vec<u8>>>>,
    calls: Mutex<Vec<(u8, Vec<u8>, usize)>>,
}

impl MockI3cDevice {
    fn new(responses: Vec<Result<Vec<u8>>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn fixed(data: Vec<u8>) -> Self {
        let responses: Vec<Result<Vec<u8>>> = (0..100).map(|_| Ok(data.clone())).collect();
        Self::new(responses)
    }

    #[allow(dead_code)]
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl I3cDevice for MockI3cDevice {
    fn write_read(&mut self, address: u8, write_buf: &[u8], read_len: usize) -> Result<Vec<u8>> {
        self.calls
            .lock()
            .unwrap()
            .push((address, write_buf.to_vec(), read_len));
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            anyhow::bail!("no more mock responses")
        }
        responses.remove(0)
    }
}

fn make_metric(
    name: &str,
    address: u8,
    data_type: DataType,
    byte_order: ByteOrder,
    scale: f64,
    offset: f64,
) -> Metric {
    Metric {
        name: name.to_string(),
        description: String::new(),
        metric_type: MetricType::Gauge,
        register_type: None,
        address: Some(address as u16),
        data_type,
        byte_order,
        scale,
        offset,
        unit: String::new(),
        command: vec![],
        response_length: None,
        response_offset: 0,
    }
}

// ═══════════════════════════════════════════════════════════════════
// 1. Config parsing — all three address modes
// ═══════════════════════════════════════════════════════════════════

#[test]
fn config_parse_pid_mode() {
    let yaml = r#"
global_labels: {}
logging: { level: info, output: stdout }
exporters: { prometheus: { enabled: true } }
collectors:
  - name: sensor-pid
    protocol:
      type: i3c
      bus: "/dev/i3c-0"
      pid: "0x0123456789AB"
    metrics:
      - name: temp
        type: gauge
        address: 0xFA
        data_type: u16
        byte_order: big_endian
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn config_parse_static_mode() {
    let yaml = r#"
global_labels: {}
logging: { level: info, output: stdout }
exporters: { prometheus: { enabled: true } }
collectors:
  - name: sensor-static
    protocol:
      type: i3c
      bus: "/dev/i3c-0"
      address: 0x30
    metrics:
      - name: humidity
        type: gauge
        address: 0x10
        data_type: u8
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn config_parse_device_class_mode() {
    let yaml = r#"
global_labels: {}
logging: { level: info, output: stdout }
exporters: { prometheus: { enabled: true } }
collectors:
  - name: sensor-class
    protocol:
      type: i3c
      bus: "/dev/i3c-0"
      device_class: "temperature-sensor"
      instance: 0
    metrics:
      - name: temp
        type: gauge
        address: 0xFA
        data_type: f32
        byte_order: big_endian
"#;
    let config: Config = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate().is_ok());
}

// ═══════════════════════════════════════════════════════════════════
// 2. Full pipeline: mock device → I3C client → read_metric → decode
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn pipeline_u8_metric() {
    let device = MockI3cDevice::fixed(vec![0x42]); // 66
    let client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );
    let client = Arc::new(tokio::sync::Mutex::new(client));
    let bus_lock = Arc::new(std::sync::Mutex::new(()));
    let metric = make_metric(
        "temp_u8",
        0xFA,
        DataType::U8,
        ByteOrder::BigEndian,
        1.0,
        0.0,
    );

    let val = bus_exporter::i3c::read_i3c_metric(&client, &metric, &bus_lock)
        .await
        .unwrap();
    assert!((val - 66.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn pipeline_u16_big_endian_with_scale_offset() {
    // 0x00F5 = 245; 245 * 0.1 + (-40.0) = -15.5
    let device = MockI3cDevice::fixed(vec![0x00, 0xF5]);
    let client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );
    let client = Arc::new(tokio::sync::Mutex::new(client));
    let bus_lock = Arc::new(std::sync::Mutex::new(()));
    let metric = make_metric(
        "temp_scaled",
        0xFA,
        DataType::U16,
        ByteOrder::BigEndian,
        0.1,
        -40.0,
    );

    let val = bus_exporter::i3c::read_i3c_metric(&client, &metric, &bus_lock)
        .await
        .unwrap();
    assert!((val - (-15.5)).abs() < 0.001);
}

#[tokio::test]
async fn pipeline_u16_little_endian() {
    // LE bytes [0x00, 0x01] → 0x0100 = 256
    let device = MockI3cDevice::fixed(vec![0x00, 0x01]);
    let client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );
    let client = Arc::new(tokio::sync::Mutex::new(client));
    let bus_lock = Arc::new(std::sync::Mutex::new(()));
    let metric = make_metric(
        "val_le",
        0x10,
        DataType::U16,
        ByteOrder::LittleEndian,
        1.0,
        0.0,
    );

    let val = bus_exporter::i3c::read_i3c_metric(&client, &metric, &bus_lock)
        .await
        .unwrap();
    assert!((val - 256.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn pipeline_f32_big_endian() {
    // IEEE 754: 42.0f32 = 0x42280000
    let device = MockI3cDevice::fixed(vec![0x42, 0x28, 0x00, 0x00]);
    let client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );
    let client = Arc::new(tokio::sync::Mutex::new(client));
    let bus_lock = Arc::new(std::sync::Mutex::new(()));
    let metric = make_metric(
        "temp_f32",
        0xFA,
        DataType::F32,
        ByteOrder::BigEndian,
        1.0,
        0.0,
    );

    let val = bus_exporter::i3c::read_i3c_metric(&client, &metric, &bus_lock)
        .await
        .unwrap();
    assert!((val - 42.0).abs() < 0.001);
}

// ═══════════════════════════════════════════════════════════════════
// 3. Multiple metrics from the same device in sequence
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn pipeline_multiple_metrics_sequential() {
    // Two reads: first returns u8=100, second returns u16=0x0200=512
    let responses: Vec<Result<Vec<u8>>> = vec![Ok(vec![0x64]), Ok(vec![0x02, 0x00])];
    let device = MockI3cDevice::new(responses);
    let client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );
    let client = Arc::new(tokio::sync::Mutex::new(client));
    let bus_lock = Arc::new(std::sync::Mutex::new(()));

    let m1 = make_metric("m1", 0x10, DataType::U8, ByteOrder::BigEndian, 1.0, 0.0);
    let m2 = make_metric("m2", 0x20, DataType::U16, ByteOrder::BigEndian, 1.0, 0.0);

    let v1 = bus_exporter::i3c::read_i3c_metric(&client, &m1, &bus_lock)
        .await
        .unwrap();
    let v2 = bus_exporter::i3c::read_i3c_metric(&client, &m2, &bus_lock)
        .await
        .unwrap();

    assert!((v1 - 100.0).abs() < f64::EPSILON);
    assert!((v2 - 512.0).abs() < f64::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════
// 4. NACK re-enumeration: first read fails, retries succeed
// ═══════════════════════════════════════════════════════════════════

#[test]
fn reenumeration_nack_then_success_static() {
    // Static address: NACK on first read, success on retry (address stays same).
    let responses: Vec<Result<Vec<u8>>> = vec![
        Err(anyhow::anyhow!("NACK: transfer error")),
        Ok(vec![0xBE, 0xEF]),
    ];
    let device = MockI3cDevice::new(responses);
    let mut client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );

    let data = client.read_register_sync(0xFA, 2).unwrap();
    assert_eq!(data, vec![0xBE, 0xEF]);

    // Verify device was called at least twice (initial + retry)
    // We can't access mock directly after move, but success proves retry worked.
}

#[test]
fn reenumeration_all_nack_exhausts_retries() {
    // All reads fail with NACK — should exhaust retries and return error.
    let responses: Vec<Result<Vec<u8>>> = vec![
        Err(anyhow::anyhow!("NACK")),
        Err(anyhow::anyhow!("NACK")),
        Err(anyhow::anyhow!("NACK")),
        Err(anyhow::anyhow!("NACK")),
    ];
    let device = MockI3cDevice::new(responses);
    let mut client = I3cClient::new(
        Box::new(device),
        "/dev/i3c-0".into(),
        AddressMode::Static(0x30),
    );

    let result = client.read_register_sync(0xFA, 2);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("retries"));
}

// ═══════════════════════════════════════════════════════════════════
// 5. Config validation — invalid configs should error
// ═══════════════════════════════════════════════════════════════════

fn parse_and_validate(yaml: &str) -> Result<()> {
    let config: Config = serde_yaml::from_str(yaml)?;
    config.validate()
}

const YAML_PREFIX: &str = r#"
global_labels: {}
logging: { level: info, output: stdout }
exporters: { prometheus: { enabled: true } }
collectors:
  - name: test
    protocol:
      type: i3c
      bus: "/dev/i3c-0"
"#;

#[test]
fn config_reject_multiple_address_modes() {
    let yaml = format!(
        "{}      pid: \"0x0123456789AB\"\n      address: 0x30\n    metrics:\n      - name: t\n        type: gauge\n        address: 0xFA\n        data_type: u16\n",
        YAML_PREFIX
    );
    let err = parse_and_validate(&yaml).unwrap_err().to_string();
    assert!(err.contains("exactly one address mode"), "got: {err}");
}

#[test]
fn config_reject_no_address_mode() {
    let yaml = format!(
        "{}    metrics:\n      - name: t\n        type: gauge\n        address: 0xFA\n        data_type: u16\n",
        YAML_PREFIX
    );
    let err = parse_and_validate(&yaml).unwrap_err().to_string();
    assert!(err.contains("exactly one address mode"), "got: {err}");
}

#[test]
fn config_reject_out_of_range_address() {
    let yaml = format!(
        "{}      address: 0x05\n    metrics:\n      - name: t\n        type: gauge\n        address: 0xFA\n        data_type: u16\n",
        YAML_PREFIX
    );
    let err = parse_and_validate(&yaml).unwrap_err().to_string();
    assert!(err.contains("0x08"), "got: {err}");
}

#[test]
fn config_reject_invalid_pid() {
    let yaml = format!(
        "{}      pid: \"0xZZZZ\"\n    metrics:\n      - name: t\n        type: gauge\n        address: 0xFA\n        data_type: u16\n",
        YAML_PREFIX
    );
    let err = parse_and_validate(&yaml).unwrap_err().to_string();
    assert!(err.contains("PID"), "got: {err}");
}

#[test]
fn config_reject_mid_endian_byte_order() {
    let yaml = format!(
        "{}      address: 0x30\n    metrics:\n      - name: t\n        type: gauge\n        address: 0xFA\n        data_type: u32\n        byte_order: mid_big_endian\n",
        YAML_PREFIX
    );
    let err = parse_and_validate(&yaml).unwrap_err().to_string();
    assert!(err.contains("mid-endian"), "got: {err}");
}
