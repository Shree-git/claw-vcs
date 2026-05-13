//! Network sync, daemon services, and protocol support for Claw VCS.
//!
//! `claw-sync` exposes gRPC services for object transfer, refs, intents,
//! changes, capsules, workstreams, and event streams. It also contains protocol
//! negotiation and security scaffolding used by the daemon.
//!
/// Revision ancestry helpers used by sync.
pub mod ancestry;
/// Capsule service implementation.
pub mod capsule_service;
/// Change service implementation.
pub mod change_service;
/// Sync client implementation.
pub mod client;
/// Version compatibility helpers.
pub mod compat;
/// Sync error types.
pub mod error;
/// Event stream service and internal event bus.
pub mod event_service;
/// HTTP client transport used for hosted remotes.
pub mod http_client;
/// Intent service implementation.
pub mod intent_service;
/// Object graph negotiation helpers.
pub mod negotiation;
/// Partial clone filters.
pub mod partial_clone;
/// Sync protocol constants and capability helpers.
pub mod protocol;
/// Authorization, audit, rate-limit, and replay helpers.
pub mod security;
/// Object/ref sync service implementation.
pub mod server;
/// Remote transport configuration.
pub mod transport;
/// Workstream service implementation.
pub mod workstream_service;

/// Generated gRPC and protobuf modules.
pub mod proto {
    /// Common generated protobuf messages.
    pub mod common {
        tonic::include_proto!("claw.common");
    }
    /// Generated object protobuf messages.
    pub mod objects {
        tonic::include_proto!("claw.objects");
    }
    /// Generated sync service protobuf messages.
    pub mod sync {
        tonic::include_proto!("claw.sync");
    }
    /// Generated intent service protobuf messages.
    pub mod intent {
        tonic::include_proto!("claw.intent");
    }
    /// Generated change service protobuf messages.
    pub mod change {
        tonic::include_proto!("claw.change");
    }
    /// Generated capsule service protobuf messages.
    pub mod capsule {
        tonic::include_proto!("claw.capsule");
    }
    /// Generated workstream service protobuf messages.
    pub mod workstream {
        tonic::include_proto!("claw.workstream");
    }
    /// Generated event service protobuf messages.
    pub mod event {
        tonic::include_proto!("claw.event");
    }
}

pub use error::SyncError;
