use super::TcpClient;
use crate::modbus::{ModbusConnection, ModbusReader};
use std::net::SocketAddr;

#[test]
fn test_tcp_client_new_not_connected() {
    let addr: SocketAddr = "127.0.0.1:502".parse().unwrap();
    let client = TcpClient::new(addr, 1);
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_tcp_client_read_without_connect_fails() {
    let addr: SocketAddr = "127.0.0.1:502".parse().unwrap();
    let mut client = TcpClient::new(addr, 1);
    let result = client.read_holding_registers(0, 1).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not connected"));
}

#[tokio::test]
async fn test_tcp_client_connect_to_invalid_endpoint_fails() {
    let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
    let mut client = TcpClient::new(addr, 1);
    let result = client.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tcp_client_disconnect() {
    let addr: SocketAddr = "127.0.0.1:502".parse().unwrap();
    let mut client = TcpClient::new(addr, 1);
    // disconnect when not connected is fine
    client.disconnect().await.unwrap();
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_tcp_count_validation() {
    let addr: SocketAddr = "127.0.0.1:502".parse().unwrap();
    let mut client = TcpClient::new(addr, 1);
    // Should fail on count validation before needing a connection
    assert!(client.read_holding_registers(0, 0).await.is_err());
    assert!(client.read_holding_registers(0, 126).await.is_err());
    assert!(client.read_coils(0, 2001).await.is_err());
}
