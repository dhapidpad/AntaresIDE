use std::path::PathBuf;
use thiserror::Error;

/// Typed errors produced by the Antares agent layer.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// An explicitly configured binary path does not exist or is not a regular file.
    #[error("configured binary path does not exist or is not a file: {0}")]
    InvalidBinaryPath(PathBuf),

    /// No Octomind executable was found in configured path or PATH.
    #[error("Octomind executable was not found")]
    BinaryNotFound,

    /// Failed to spawn the child process.
    #[error("failed to spawn agent child process: {0}")]
    ProcessSpawnFailed(String),

    /// The child process exited prematurely or with an error status.
    #[error("agent process exited unexpectedly with status: {0:?}")]
    ProcessExited(Option<i32>),

    /// General I/O failure communicating with the process pipes.
    #[error("I/O error communicating with agent: {0}")]
    IoError(String),

    /// Failed to serialize or deserialize JSON-RPC message.
    #[error("JSON-RPC serialization/deserialization error: {0}")]
    SerializationError(String),

    /// Malformed JSON-RPC protocol message or invalid version.
    #[error("JSON-RPC protocol error: {0}")]
    ProtocolError(String),

    /// Server returned an explicit JSON-RPC error object.
    #[error("agent returned RPC error {code}: {message}")]
    RpcError {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },

    /// An operation timed out waiting for child process response.
    #[error("agent operation timed out: {0}")]
    Timeout(String),

    /// Internal communication channel closed unexpectedly.
    #[error("agent communication channel closed")]
    ChannelClosed,

    /// The specified session ID was not found or has been closed.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// The operation was explicitly cancelled.
    #[error("agent turn was cancelled")]
    Cancelled,

    /// Unexpected internal error state.
    #[error("internal agent error: {0}")]
    Internal(String),
}
