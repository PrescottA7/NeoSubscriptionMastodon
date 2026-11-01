//! Settings shared by more than one outbound protocol.

pub mod mux;
pub mod quic;
pub mod tls;
pub mod transport;

pub use mux::MuxConfig;
pub use quic::QuicConfig;
pub use tls::{RealityConfig, TlsConfig};
pub use transport::{Transport, TransportKind};

/// Default tag applied to the generated outbound in client configs.
pub const DEFAULT_TAG: &str = "nsm-node";
