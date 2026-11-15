//! IPC client communicating with the AuraZoneRipple background daemon over Named Pipe.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\AuraZoneRipple_IPC";

/// Resolves the pipe name from environment or returns default.
pub fn get_pipe_name() -> String {
    std::env::var("AURAZONE_PIPE_NAME").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_string())
}

/// Sends a command to the default/configured daemon named pipe and returns the response.
pub fn send_ipc_command(command: &str) -> Result<String, std::io::Error> {
    let pipe_name = get_pipe_name();
    send_command_to_pipe(&pipe_name, command)
}

/// Connects to a specific named pipe with retry on transient ERROR_PIPE_BUSY (231)
/// and executes a request-response transaction.
pub fn send_command_to_pipe(pipe_name: &str, command: &str) -> Result<String, std::io::Error> {
    let timeout = Duration::from_secs(3);
    let start = Instant::now();

    let mut pipe = loop {
        match OpenOptions::new().read(true).write(true).open(pipe_name) {
            Ok(file) => break file,
            Err(e) if e.raw_os_error() == Some(231) => {
                // Win32 ERROR_PIPE_BUSY: Server is recycling instances
                if start.elapsed() >= timeout {
                    return Err(e);
                }
                thread::sleep(Duration::from_millis(15));
            }
            Err(e) => return Err(e),
        }
    };

    let formatted = if command.ends_with('\n') {
        command.to_string()
    } else {
        format!("{}\n", command.trim_end())
    };

    pipe.write_all(formatted.as_bytes())?;
    pipe.flush()?;

    let mut reader = BufReader::new(pipe);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    Ok(response)
}
