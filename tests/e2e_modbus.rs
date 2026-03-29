//! E2E integration test: Modbus TCP simulator → bus-exporter pull → JSON validation.
//!
//! Uses the shared test harness from `tests/common/mod.rs` for fixtures,
//! config generation, pull execution, and validation. Only the Modbus TCP
//! simulator (protocol-specific mock) lives here.

mod common;

use std::collections::HashMap;
use std::future;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_modbus::prelude::*;
use tokio_modbus::server::tcp::{accept_tcp_connection, Server};
use tokio_modbus::server::Service;

use common::{standard_fixtures, ConnectionParams, TestFixtures};

// ── Modbus TCP Simulator ──────────────────────────────────────────────

/// Register store populated from shared test fixtures.
#[derive(Clone)]
struct SimulatorService {
    holding: Arc<HashMap<u16, u16>>,
    input: Arc<HashMap<u16, u16>>,
    coils: Arc<HashMap<u16, bool>>,
}

impl SimulatorService {
    /// Build register maps from the shared test fixtures.
    fn from_fixtures(fixtures: &TestFixtures) -> Self {
        let mut holding = HashMap::new();
        let mut input = HashMap::new();
        let mut coils = HashMap::new();

        for m in &fixtures.metrics {
            match m.register_type {
                "holding" => {
                    for (i, &val) in m.raw_registers.iter().enumerate() {
                        holding.insert(m.address + i as u16, val);
                    }
                }
                "input" => {
                    for (i, &val) in m.raw_registers.iter().enumerate() {
                        input.insert(m.address + i as u16, val);
                    }
                }
                "coil" => {
                    for (i, &val) in m.raw_registers.iter().enumerate() {
                        coils.insert(m.address + i as u16, val != 0);
                    }
                }
                _ => {}
            }
        }

        Self {
            holding: Arc::new(holding),
            input: Arc::new(input),
            coils: Arc::new(coils),
        }
    }

    fn read_holding(&self, addr: u16, count: u16) -> Vec<u16> {
        (addr..addr + count)
            .map(|a| self.holding.get(&a).copied().unwrap_or(0))
            .collect()
    }

    fn read_input(&self, addr: u16, count: u16) -> Vec<u16> {
        (addr..addr + count)
            .map(|a| self.input.get(&a).copied().unwrap_or(0))
            .collect()
    }

    fn read_coils(&self, addr: u16, count: u16) -> Vec<bool> {
        (addr..addr + count)
            .map(|a| self.coils.get(&a).copied().unwrap_or(false))
            .collect()
    }
}

impl Service for SimulatorService {
    type Request = Request<'static>;
    type Response = Response;
    type Exception = Exception;
    type Future = future::Ready<Result<Self::Response, Self::Exception>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let resp = match req {
            Request::ReadHoldingRegisters(addr, count) => {
                Response::ReadHoldingRegisters(self.read_holding(addr, count))
            }
            Request::ReadInputRegisters(addr, count) => {
                Response::ReadInputRegisters(self.read_input(addr, count))
            }
            Request::ReadCoils(addr, count) => Response::ReadCoils(self.read_coils(addr, count)),
            Request::ReadDiscreteInputs(_, count) => {
                Response::ReadDiscreteInputs(vec![false; count as usize])
            }
            _ => return future::ready(Err(Exception::IllegalFunction)),
        };
        future::ready(Ok(resp))
    }
}

/// Start the Modbus TCP simulator on an OS-assigned port. Returns the bound address.
async fn start_simulator(fixtures: &TestFixtures) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = Server::new(listener);
    let service = SimulatorService::from_fixtures(fixtures);

    let handle = tokio::spawn(async move {
        let new_service = Arc::new(move |_socket_addr: SocketAddr| {
            let svc = service.clone();
            Ok(Some(svc)) as io::Result<Option<SimulatorService>>
        });
        let on_connected = |stream: tokio::net::TcpStream, socket_addr: SocketAddr| {
            let ns = Arc::clone(&new_service);
            async move { accept_tcp_connection(stream, socket_addr, &*ns) }
        };
        let _ = server
            .serve(&on_connected, |err: io::Error| {
                eprintln!("simulator process error: {err}");
            })
            .await;
    });

    (addr, handle)
}

// ── Test ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn e2e_modbus_tcp_pull() {
    let fixtures = standard_fixtures();

    // 1. Start simulator populated from shared fixtures
    let (sim_addr, sim_handle) = start_simulator(&fixtures).await;

    // 2. Generate config using shared harness
    let tmp = tempfile::tempdir().unwrap();
    let connection = ConnectionParams::ModbusTcp {
        endpoint: format!("{}:{}", sim_addr.ip(), sim_addr.port()),
        slave_id: 1,
    };
    let config_path = common::generate_config(tmp.path(), "test_device", &connection, &fixtures);

    // 3. Run pull
    let result = common::run_pull(&config_path).await;
    assert_eq!(
        result.exit_code,
        Some(0),
        "pull failed:\nstderr: {}",
        result.stderr
    );

    // 4. Validate results
    common::validate(&result, &fixtures);

    // Cleanup
    sim_handle.abort();
}
