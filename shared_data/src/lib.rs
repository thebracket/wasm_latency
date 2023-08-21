//! Support data for a web/wasm latency under load measuring system.
//! This crate contains shared data structures and library functions.
//!
//! The theory is:
//!
//! (Client) sends "latency request"
//! (Server) replies with the server time
//! (Client) pings back with both the server time and the client time
//! (Server) replies back with server time, client time, and the new server time (ack time)
//! (Client) appends the time it receives this
//!
//! You now have everything you need to calculate round-trip latency, without trusting
//! either clock.
//!
//! `Latency of M = (server_ack_ts - server_ts) - ((client_ack_ts - client_ts) * 0.5)`
//!
//! See [this document](https://ankitbko.github.io/blog/2022/06/websocket-latency/)

use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Helper function to get the current time in ms since the UNIX epoch.
/// This corresponds to JavaScript's `now()` function.
#[cfg(not(target_arch = "wasm32"))]
pub fn unix_now_ms() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(t) => t.as_millis(),
        Err(_e) => 0,
    }
}

/// Helper function to get the current time in ms since the UNIX epoch.
/// This corresponds to JavaScript's `now()` function. (WASM version)
#[cfg(target_arch = "wasm32")]
pub fn unix_now_ms() -> u128 {
    use web_time::SystemTime;
    match SystemTime::now().duration_since(web_time::UNIX_EPOCH) {
        Ok(t) => t.as_millis(),
        Err(_e) => 0,
    }
}

pub const MAGIC_NUMBER: u16 = 0xBE47;
const SIZE_U16: usize = std::mem::size_of::<u16>();
const HEADER_SIZE: usize = SIZE_U16 * 2;
const SIZE_U128: usize = std::mem::size_of::<u128>();

#[derive(Debug, PartialEq)]
pub enum LatencyTest {
    InitialRequest {
        magic: u16,
    },
    FirstReply {
        magic: u16,
        server_time: u128,
    },
    FirstResponse {
        magic: u16,
        server_time: u128,
        client_time: u128,
    },
    SecondReply {
        magic: u16,
        server_time: u128,
        client_time: u128,
        server_ack_time: u128,
    },
    Final {
        magic: u16,
        server_time: u128,
        client_time: u128,
        server_ack_time: u128,
        client_ack_time: u128,
    },
}

impl LatencyTest {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        match self {
            LatencyTest::InitialRequest { magic } => {
                buf.extend(magic.to_be_bytes());
                buf.extend((1u16).to_be_bytes());
            }
            LatencyTest::FirstReply { magic, server_time } => {
                buf.extend(magic.to_be_bytes());
                buf.extend((2u16).to_be_bytes());
                buf.extend(server_time.to_be_bytes());
            }
            LatencyTest::FirstResponse {
                magic,
                server_time,
                client_time,
            } => {
                buf.extend(magic.to_be_bytes());
                buf.extend((3u16).to_be_bytes());
                buf.extend(server_time.to_be_bytes());
                buf.extend(client_time.to_be_bytes());
            }
            LatencyTest::SecondReply {
                magic,
                server_time,
                client_time,
                server_ack_time,
            } => {
                buf.extend(magic.to_be_bytes());
                buf.extend((4u16).to_be_bytes());
                buf.extend(server_time.to_be_bytes());
                buf.extend(client_time.to_be_bytes());
                buf.extend(server_ack_time.to_be_bytes());
            }
            LatencyTest::Final {
                magic,
                server_time,
                client_time,
                server_ack_time,
                client_ack_time,
            } => {
                buf.extend(magic.to_be_bytes());
                buf.extend((5u16).to_be_bytes());
                buf.extend(server_time.to_be_bytes());
                buf.extend(client_time.to_be_bytes());
                buf.extend(server_ack_time.to_be_bytes());
                buf.extend(client_ack_time.to_be_bytes());
            }
        }

        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, LatencyTestError> {
        let magic = u16::from_be_bytes(bytes[0..2].try_into().map_err(|_| LatencyTestError::Read)?);
        if magic != MAGIC_NUMBER {
            return Err(LatencyTestError::InvalidMagic);
        }

        let req = u16::from_be_bytes(bytes[2..4].try_into().map_err(|_| LatencyTestError::Read)?);
        match req {
            1 => Ok(Self::InitialRequest { magic }),
            2 => {
                let server_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE..HEADER_SIZE + SIZE_U128]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                Ok(Self::FirstReply { magic, server_time })
            }
            3 => {
                let server_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE..HEADER_SIZE + SIZE_U128]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let client_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + SIZE_U128..HEADER_SIZE + (SIZE_U128 * 2)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                Ok(Self::FirstResponse {
                    magic,
                    server_time,
                    client_time,
                })
            }
            4 => {
                let server_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE..HEADER_SIZE + SIZE_U128]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let client_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + SIZE_U128..HEADER_SIZE + (SIZE_U128 * 2)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let server_ack_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + (SIZE_U128 * 2)..HEADER_SIZE + (SIZE_U128 * 3)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                Ok(Self::SecondReply {
                    magic,
                    server_time,
                    client_time,
                    server_ack_time,
                })
            }
            5 => {
                let server_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE..HEADER_SIZE + SIZE_U128]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let client_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + SIZE_U128..HEADER_SIZE + (SIZE_U128 * 2)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let server_ack_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + (SIZE_U128 * 2)..HEADER_SIZE + (SIZE_U128 * 3)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                let client_ack_time = u128::from_be_bytes(
                    bytes[HEADER_SIZE + (SIZE_U128 * 3)..HEADER_SIZE + (SIZE_U128 * 4)]
                        .try_into()
                        .map_err(|_| LatencyTestError::Read)?,
                );
                Ok(Self::Final {
                    magic,
                    server_time,
                    client_time,
                    server_ack_time,
                    client_ack_time,
                })
            }
            _ => Err(LatencyTestError::BadRequest),
        }
    }

    pub fn calculate_latency(&self) -> (f64, f64, f64) {
        match self {
            LatencyTest::Final {
                server_time,
                client_time,
                server_ack_time,
                client_ack_time,
                ..
            } => {
                let server_latency = (server_ack_time - server_time) as f64;
                let client_latency = (client_ack_time - client_time) as f64;
                let latency = server_latency - (client_latency * 0.5);
                (latency, server_latency , client_latency)
            }
            _ => (0., 0., 0.),
        }
    }
}

#[derive(Error, Debug)]
pub enum LatencyTestError {
    #[error("Error reading byte data")]
    Read,
    #[error("Invalid magic number")]
    InvalidMagic,
    #[error("Bad request number")]
    BadRequest,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_decode_initial() {
        let original = LatencyTest::InitialRequest {
            magic: MAGIC_NUMBER,
        };
        let bytes = original.encode();
        let decoded = LatencyTest::decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn encode_decode_first_reply() {
        let original = LatencyTest::FirstReply {
            magic: MAGIC_NUMBER,
            server_time: unix_now_ms(),
        };
        let bytes = original.encode();
        let decoded = LatencyTest::decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn encode_decode_first_response() {
        let original = LatencyTest::FirstResponse {
            magic: MAGIC_NUMBER,
            server_time: unix_now_ms(),
            client_time: unix_now_ms() + 30,
        };
        let bytes = original.encode();
        let decoded = LatencyTest::decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn encode_decode_second_reply() {
        let original = LatencyTest::SecondReply {
            magic: MAGIC_NUMBER,
            server_time: unix_now_ms(),
            client_time: unix_now_ms() + 30,
            server_ack_time: unix_now_ms() + 60,
        };
        let bytes = original.encode();
        let decoded = LatencyTest::decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn encode_decode_final() {
        let original = LatencyTest::Final {
            magic: MAGIC_NUMBER,
            server_time: unix_now_ms(),
            client_time: unix_now_ms() + 30,
            server_ack_time: unix_now_ms() + 60,
            client_ack_time: unix_now_ms() + 90,
        };
        let bytes = original.encode();
        let decoded = LatencyTest::decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
