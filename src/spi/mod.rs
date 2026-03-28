use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config;
use crate::decoder;

/// Type alias for per-device mutex (different chip-selects are independent).
pub type DeviceLock = Arc<tokio::sync::Mutex<()>>;

/// Trait abstracting SPI device operations for testability.
pub trait SpiDevice: Send {
    /// Perform a full-duplex SPI transfer: send tx_buf, return rx_buf of same length.
    fn transfer(&mut self, tx_buf: &[u8]) -> Result<Vec<u8>>;
}

/// Real SPI device using Linux spidev.
#[cfg(target_os = "linux")]
pub mod linux_device {
    use super::*;

    pub struct LinuxSpiDevice {
        device_path: String,
        speed_hz: u32,
        mode: u8,
        bits_per_word: u8,
        inner: Option<spidev::Spidev>,
    }

    impl LinuxSpiDevice {
        pub fn new(device_path: String, speed_hz: u32, mode: u8, bits_per_word: u8) -> Self {
            Self {
                device_path,
                speed_hz,
                mode,
                bits_per_word,
                inner: None,
            }
        }

        pub fn open(&mut self) -> Result<()> {
            use spidev::{Spidev, SpidevOptions, SpiModeFlags};

            let mut spi = Spidev::open(&self.device_path)
                .with_context(|| format!("opening SPI device {}", self.device_path))?;

            let mode_flags = match self.mode {
                0 => SpiModeFlags::SPI_MODE_0,
                1 => SpiModeFlags::SPI_MODE_1,
                2 => SpiModeFlags::SPI_MODE_2,
                3 => SpiModeFlags::SPI_MODE_3,
                _ => anyhow::bail!("invalid SPI mode: {}", self.mode),
            };

            let options = SpidevOptions::new()
                .bits_per_word(self.bits_per_word)
                .max_speed_hz(self.speed_hz)
                .mode(mode_flags)
                .build();

            spi.configure(&options)
                .with_context(|| format!("configuring SPI device {}", self.device_path))?;

            self.inner = Some(spi);
            Ok(())
        }
    }

    impl SpiDevice for LinuxSpiDevice {
        fn transfer(&mut self, tx_buf: &[u8]) -> Result<Vec<u8>> {
            use spidev::SpidevTransfer;

            let spi = self
                .inner
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("SPI device not opened"))?;

            let mut rx_buf = vec![0u8; tx_buf.len()];
            let mut transfer = SpidevTransfer::read_write(tx_buf, &mut rx_buf);
            spi.transfer(&mut transfer)
                .context("SPI transfer failed")?;
            Ok(rx_buf)
        }
    }
}

/// Stub SPI device for non-Linux platforms.
pub struct StubSpiDevice;

impl SpiDevice for StubSpiDevice {
    fn transfer(&mut self, _tx_buf: &[u8]) -> Result<Vec<u8>> {
        anyhow::bail!("StubSpiDevice: no real SPI hardware available")
    }
}

/// SPI client wrapping a device for async read operations.
pub struct SpiClient {
    device: Arc<std::sync::Mutex<Box<dyn SpiDevice>>>,
    device_path: String,
    connected: bool,
}

/// Per-device mutex map for serializing access to same chip-select.
static DEVICE_LOCKS: std::sync::LazyLock<std::sync::Mutex<HashMap<String, DeviceLock>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Get or create a per-device lock.
pub fn get_device_lock(device_path: &str) -> DeviceLock {
    let mut map = DEVICE_LOCKS.lock().unwrap();
    map.entry(device_path.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

impl SpiClient {
    pub fn new(device: Box<dyn SpiDevice>, device_path: String) -> Self {
        Self {
            device: Arc::new(std::sync::Mutex::new(device)),
            device_path,
            connected: false,
        }
    }

    /// Perform a synchronous SPI transfer.
    pub fn transfer_sync(&self, tx_buf: &[u8]) -> Result<Vec<u8>> {
        let mut dev = self
            .device
            .lock()
            .map_err(|e| anyhow::anyhow!("device lock poisoned: {e}"))?;
        dev.transfer(tx_buf)
    }
}

/// Read a single SPI metric.
pub async fn read_spi_metric(
    client: &SpiClient,
    metric: &config::Metric,
    device_lock: &DeviceLock,
) -> Result<f64> {
    let data_type = map_data_type(metric.data_type);
    let byte_order = map_byte_order(metric.byte_order);

    // Reject mid-endian byte orders for SPI
    if matches!(
        metric.byte_order,
        config::ByteOrder::MidBigEndian | config::ByteOrder::MidLittleEndian
    ) {
        anyhow::bail!(
            "mid-endian byte order is not supported for SPI protocol (Modbus-specific)"
        );
    }

    if metric.command.is_empty() {
        anyhow::bail!("SPI metric '{}': command must not be empty", metric.name);
    }

    let response_length = metric
        .response_length
        .unwrap_or(metric.command.len() as u16) as usize;
    let response_offset = metric.response_offset as usize;
    let num_bytes = decoder::byte_count(data_type);

    if response_offset + num_bytes > response_length {
        anyhow::bail!(
            "SPI metric '{}': response_offset ({}) + data bytes ({}) exceeds response_length ({})",
            metric.name,
            response_offset,
            num_bytes,
            response_length
        );
    }

    // Build TX buffer: command bytes, zero-padded to response_length
    let mut tx_buf = metric.command.clone();
    if tx_buf.len() < response_length {
        tx_buf.resize(response_length, 0);
    }

    let scale = metric.scale;
    let offset = metric.offset;

    let device = Arc::clone(&client.device);
    let device_lock = device_lock.clone();
    let device_path = client.device_path.clone();

    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let _lock = device_lock.blocking_lock();
        let mut dev = device
            .lock()
            .map_err(|e| anyhow::anyhow!("device lock poisoned: {e}"))?;
        dev.transfer(&tx_buf)
            .with_context(|| format!("SPI transfer on {}", device_path))
    })
    .await
    .context("spawn_blocking join error")??;

    // Extract payload from response at offset
    let payload = &bytes[response_offset..response_offset + num_bytes];

    decoder::decode_bytes(payload, data_type, byte_order, scale, offset)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn map_byte_order(bo: config::ByteOrder) -> decoder::ByteOrder {
    match bo {
        config::ByteOrder::BigEndian => decoder::ByteOrder::BigEndian,
        config::ByteOrder::LittleEndian => decoder::ByteOrder::LittleEndian,
        config::ByteOrder::MidBigEndian => decoder::ByteOrder::MidBigEndian,
        config::ByteOrder::MidLittleEndian => decoder::ByteOrder::MidLittleEndian,
    }
}

fn map_data_type(dt: config::DataType) -> decoder::DataType {
    match dt {
        config::DataType::U8 => decoder::DataType::U8,
        config::DataType::U16 => decoder::DataType::U16,
        config::DataType::I16 => decoder::DataType::I16,
        config::DataType::U32 => decoder::DataType::U32,
        config::DataType::I32 => decoder::DataType::I32,
        config::DataType::F32 => decoder::DataType::F32,
        config::DataType::U64 => decoder::DataType::U64,
        config::DataType::I64 => decoder::DataType::I64,
        config::DataType::F64 => decoder::DataType::F64,
        config::DataType::Bool => decoder::DataType::Bool,
    }
}

/// Connection/lifecycle trait impl for SpiClient (mirrors ModbusConnection).
#[async_trait]
impl crate::modbus::ModbusConnection for SpiClient {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
