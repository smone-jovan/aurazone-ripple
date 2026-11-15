//! IPC Command protocol definitions, parsing, and response formatting.

use std::str::FromStr;

/// Commands accepted over the Windows Named Pipe IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    Status,
    Toggle,
    Stop,
    Reload,
    Unknown(String),
}

impl FromStr for IpcCommand {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let cmd = match trimmed.to_ascii_uppercase().as_str() {
            "STATUS" => IpcCommand::Status,
            "TOGGLE" => IpcCommand::Toggle,
            "STOP" => IpcCommand::Stop,
            "RELOAD" => IpcCommand::Reload,
            _ => IpcCommand::Unknown(trimmed.to_string()),
        };
        Ok(cmd)
    }
}

impl IpcCommand {
    pub fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }
}

/// Standardized responses emitted over the Named Pipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcResponse {
    Status { active: bool, preset: String },
    Toggle { active: bool },
    Stop,
    Reload,
    Error(String),
}

impl IpcResponse {
    pub fn to_protocol_string(&self) -> String {
        match self {
            IpcResponse::Status { active, preset } => {
                format!("OK running active={} preset={}\n", active, preset)
            }
            IpcResponse::Toggle { active } => {
                if *active {
                    "OK TOGGLE active\n".to_string()
                } else {
                    "OK TOGGLE paused\n".to_string()
                }
            }
            IpcResponse::Stop => "OK STOP\n".to_string(),
            IpcResponse::Reload => "OK RELOAD\n".to_string(),
            IpcResponse::Error(msg) => format!("ERR: {}\n", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commands() {
        assert_eq!(IpcCommand::parse("STATUS"), IpcCommand::Status);
        assert_eq!(IpcCommand::parse("status\n"), IpcCommand::Status);
        assert_eq!(IpcCommand::parse("  TOGGLE  \r\n"), IpcCommand::Toggle);
        assert_eq!(IpcCommand::parse("STOP"), IpcCommand::Stop);
        assert_eq!(IpcCommand::parse("RELOAD"), IpcCommand::Reload);
        assert_eq!(
            IpcCommand::parse("GARBAGE_PAYLOAD"),
            IpcCommand::Unknown("GARBAGE_PAYLOAD".to_string())
        );
    }

    #[test]
    fn test_response_strings() {
        let resp_status = IpcResponse::Status {
            active: true,
            preset: "Cyberpunk".to_string(),
        };
        assert_eq!(
            resp_status.to_protocol_string(),
            "OK running active=true preset=Cyberpunk\n"
        );

        let resp_pause = IpcResponse::Toggle { active: false };
        assert_eq!(resp_pause.to_protocol_string(), "OK TOGGLE paused\n");

        let resp_active = IpcResponse::Toggle { active: true };
        assert_eq!(resp_active.to_protocol_string(), "OK TOGGLE active\n");

        let resp_stop = IpcResponse::Stop;
        assert_eq!(resp_stop.to_protocol_string(), "OK STOP\n");

        let resp_err = IpcResponse::Error("Unknown command".to_string());
        assert_eq!(resp_err.to_protocol_string(), "ERR: Unknown command\n");
    }
}
