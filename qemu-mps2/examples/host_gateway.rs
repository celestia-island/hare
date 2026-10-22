//! Host-side Gateway demo — connects to QEMU's serial output and decodes
//! hare wire protocol frames from the emulated sensor node.
//!
//! This is a Cargo example — build it with
//! `cargo build -p kei-qemu-mps2 --example host_gateway`.
//!
//! Usage:
//!   1. Build the firmware: `cargo build --release -p kei-qemu-mps2 --target thumbv7em-none-eabi`
//!   2. Run the gateway: `cargo run -p kei-qemu-mps2 --example host_gateway -- <serial-device>`
//!
//! On Linux the serial device is /dev/pts/N (printed by QEMU on stderr).
//! On Windows use a named pipe (\\.\pipe\...) or QEMU's -serial stdio
//! and pipe stdin/stdout directly.
//!
//! For the simplest demo, run `./run_demo.sh` in this directory: it builds
//! both sides and pipes QEMU's `-serial stdio` into this gateway.

use std::env;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;

#[cfg(unix)]
use std::os::unix::io::FromRawFd;

use hare::hal::{Transport, TransportError};
use hare::wire::{Gateway, Incoming};

/// A Transport backed by a raw file descriptor (the QEMU serial pipe).
struct FdTransport {
    stdin: std::io::Stdin,
    stdout: std::io::Stdout,
}

impl Transport for FdTransport {
    fn send(&mut self, data: &[u8]) -> Result<usize, TransportError> {
        self.stdout
            .write_all(data)
            .map(|_| data.len())
            .map_err(|_| TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        self.stdin.read(buf).map_err(|_| TransportError::Io)
    }
}

fn main() {
    println!("hare QEMU Gateway demo\n");
    println!("Connect this to QEMU's UART via:");
    println!("  ./run_demo.sh\n");

    let mut gateway = Gateway::new(FdTransport {
        stdin: io::stdin(),
        stdout: io::stdout(),
    });

    println!("[gateway] waiting for frames from node...\n");

    loop {
        match gateway.recv() {
            Ok(Incoming::Telemetry(t)) => {
                println!(
                    "[gateway] TELEMETRY  station={} reg=0x{:04X} value={:.1} unit={} t={}",
                    t.station_id,
                    t.register,
                    t.value,
                    t.unit.as_str(),
                    t.timestamp_ms
                );
            }
            Ok(Incoming::Status(s)) => {
                println!(
                    "[gateway] STATUS     station={} state={:?} detail={}",
                    s.station_id, s.state, s.detail
                );
            }
            Ok(Incoming::DiscoverResponse(d)) => {
                println!(
                    "[gateway] DISCOVER   station={} name={} registers={}",
                    d.station_id, d.name, d.register_count
                );
            }
            Ok(Incoming::Alarm(a)) => {
                println!(
                    "[gateway] ALARM      station={} reg=0x{:04X} level={:?} msg={}",
                    a.station_id, a.register, a.level, a.message
                );
            }
            Ok(Incoming::Nack(n)) => {
                println!(
                    "[gateway] NACK       station={} code={} msg={}",
                    n.station_id, n.error_code, n.message
                );
            }
            Err(e) => {
                eprintln!("[gateway] error: {:?}", e);
                // Don't exit — the stream may recover after re-sync.
            }
        }
    }
}
