//! Windows Named Pipe IPC Subsystem for AuraZoneRipple_Rust.

pub mod client;
pub mod mutex;
pub mod protocol;
pub mod server;

pub use client::{get_pipe_name, send_command_to_pipe, send_ipc_command};
pub use mutex::{MutexError, SingleInstanceMutex, DEFAULT_MUTEX_NAME};
pub use protocol::{IpcCommand, IpcResponse};
pub use server::{wake_up_pipe, IpcCommandHandler, NamedPipeServer, DEFAULT_PIPE_NAME};
