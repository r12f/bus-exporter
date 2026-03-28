//! Shared helpers for bus protocol modules (Modbus, I2C, SPI).

use crate::config;
use crate::decoder;

/// Map config byte order to decoder byte order.
pub fn map_byte_order(bo: config::ByteOrder) -> decoder::ByteOrder {
    match bo {
        config::ByteOrder::BigEndian => decoder::ByteOrder::BigEndian,
        config::ByteOrder::LittleEndian => decoder::ByteOrder::LittleEndian,
        config::ByteOrder::MidBigEndian => decoder::ByteOrder::MidBigEndian,
        config::ByteOrder::MidLittleEndian => decoder::ByteOrder::MidLittleEndian,
    }
}

/// Map config data type to decoder data type.
pub fn map_data_type(dt: config::DataType) -> decoder::DataType {
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
