// FEATURE: T15

//! pgcat/pgbouncer-compatible admin database surface.
//!
//! Exposes the legacy admin commands a pgbouncer-trained operator expects:
//! `SHOW POOLS`, `SHOW DATABASES`, `SHOW STATS`, `RELOAD`, `PAUSE`, `RESUME`,
//! `KILL <pid>`. Responses are formatted as PostgreSQL `RowDescription` +
//! `DataRow` text (handled by the proxy layer); this module owns the command
//! parser + state machine.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdminCommand {
    ShowPools,
    ShowDatabases,
    ShowStats,
    ShowClients,
    Reload,
    Pause,
    Resume,
    Kill(u32),
    SetVerbose(u8),
}

impl AdminCommand {
    /// Parse a SQL-style admin statement. Case-insensitive on keywords; the
    /// trailing semicolon is optional.
    pub fn parse(statement: &str) -> Result<Self, AdminError> {
        let trimmed = statement.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            return Err(AdminError::EmptyStatement);
        }
        let upper = trimmed.to_ascii_uppercase();

        if let Some(rest) = upper.strip_prefix("SHOW ") {
            return match rest.trim() {
                "POOLS" => Ok(Self::ShowPools),
                "DATABASES" => Ok(Self::ShowDatabases),
                "STATS" => Ok(Self::ShowStats),
                "CLIENTS" => Ok(Self::ShowClients),
                other => Err(AdminError::UnknownShowTarget(other.to_string())),
            };
        }

        match upper.as_str() {
            "RELOAD" => return Ok(Self::Reload),
            "PAUSE" => return Ok(Self::Pause),
            "RESUME" => return Ok(Self::Resume),
            _ => {}
        }
        if let Some(rest) = upper.strip_prefix("KILL ") {
            let pid = rest
                .trim()
                .parse::<u32>()
                .map_err(|_| AdminError::InvalidPid(rest.trim().to_string()))?;
            return Ok(Self::Kill(pid));
        }
        if let Some(rest) = upper.strip_prefix("SET VERBOSE ") {
            let level = rest
                .trim()
                .parse::<u8>()
                .map_err(|_| AdminError::InvalidVerboseLevel(rest.trim().to_string()))?;
            return Ok(Self::SetVerbose(level));
        }

        match upper.as_str() {
            "RELOAD" => Ok(Self::Reload),
            "PAUSE" => Ok(Self::Pause),
            "RESUME" => Ok(Self::Resume),
            other => Err(AdminError::UnknownCommand(other.to_string())),
        }
    }
}

/// Lifecycle state for the admin DB. SIGHUP-driven config reload + manual
/// pause/resume are tracked here.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdminState {
    pub paused: bool,
    pub generation: u64,
    pub reloads: u64,
    pub kills: u64,
}

impl Default for AdminState {
    fn default() -> Self {
        Self {
            paused: false,
            generation: 1,
            reloads: 0,
            kills: 0,
        }
    }
}

impl AdminState {
    pub fn apply(&mut self, command: &AdminCommand) -> Result<AdminAck, AdminError> {
        match command {
            AdminCommand::ShowPools
            | AdminCommand::ShowDatabases
            | AdminCommand::ShowStats
            | AdminCommand::ShowClients => Ok(AdminAck::Show),
            AdminCommand::Reload => {
                self.reloads += 1;
                self.generation += 1;
                Ok(AdminAck::Reloaded {
                    generation: self.generation,
                })
            }
            AdminCommand::Pause => {
                if self.paused {
                    return Err(AdminError::AlreadyPaused);
                }
                self.paused = true;
                Ok(AdminAck::Paused)
            }
            AdminCommand::Resume => {
                if !self.paused {
                    return Err(AdminError::NotPaused);
                }
                self.paused = false;
                Ok(AdminAck::Resumed)
            }
            AdminCommand::Kill(pid) => {
                self.kills += 1;
                Ok(AdminAck::Killed { pid: *pid })
            }
            AdminCommand::SetVerbose(level) => Ok(AdminAck::VerboseLevelSet { level: *level }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdminAck {
    Show,
    Reloaded { generation: u64 },
    Paused,
    Resumed,
    Killed { pid: u32 },
    VerboseLevelSet { level: u8 },
}

impl AdminAck {
    pub fn command_complete_tag(&self) -> String {
        match self {
            Self::Show => "SHOW".to_string(),
            Self::Reloaded { generation } => format!("RELOAD {generation}"),
            Self::Paused => "PAUSE".to_string(),
            Self::Resumed => "RESUME".to_string(),
            Self::Killed { pid } => format!("KILL {pid}"),
            Self::VerboseLevelSet { level } => format!("SET VERBOSE {level}"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AdminError {
    AlreadyPaused,
    EmptyStatement,
    InvalidPid(String),
    InvalidVerboseLevel(String),
    NotPaused,
    UnknownCommand(String),
    UnknownShowTarget(String),
}

impl fmt::Display for AdminError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyPaused => write!(formatter, "pool is already paused"),
            Self::EmptyStatement => write!(formatter, "admin statement is empty"),
            Self::InvalidPid(value) => write!(formatter, "invalid PID: {value}"),
            Self::InvalidVerboseLevel(value) => {
                write!(formatter, "invalid verbose level: {value}")
            }
            Self::NotPaused => write!(formatter, "pool is not paused"),
            Self::UnknownCommand(value) => write!(formatter, "unknown admin command: {value}"),
            Self::UnknownShowTarget(value) => {
                write!(formatter, "unknown SHOW target: {value}")
            }
        }
    }
}

impl Error for AdminError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_show_pools() {
        assert_eq!(
            AdminCommand::parse("SHOW POOLS;"),
            Ok(AdminCommand::ShowPools)
        );
        assert_eq!(
            AdminCommand::parse("show pools"),
            Ok(AdminCommand::ShowPools)
        );
    }

    #[test]
    fn parse_kill_with_pid() {
        assert_eq!(
            AdminCommand::parse("KILL 4242"),
            Ok(AdminCommand::Kill(4242))
        );
    }

    #[test]
    fn parse_rejects_invalid_pid() {
        assert!(matches!(
            AdminCommand::parse("KILL foo"),
            Err(AdminError::InvalidPid(_))
        ));
    }

    #[test]
    fn parse_unknown_command() {
        assert!(matches!(
            AdminCommand::parse("DROP TABLE x"),
            Err(AdminError::UnknownCommand(_))
        ));
    }

    #[test]
    fn parse_empty_rejected() {
        assert_eq!(AdminCommand::parse("   "), Err(AdminError::EmptyStatement));
    }

    #[test]
    fn reload_bumps_generation() {
        let mut state = AdminState::default();
        let ack = state.apply(&AdminCommand::Reload).expect("reload");
        assert_eq!(ack, AdminAck::Reloaded { generation: 2 });
        assert_eq!(state.reloads, 1);
        assert_eq!(ack.command_complete_tag(), "RELOAD 2");
    }

    #[test]
    fn pause_resume_roundtrip() {
        let mut state = AdminState::default();
        assert_eq!(state.apply(&AdminCommand::Pause), Ok(AdminAck::Paused));
        assert!(state.paused);
        assert_eq!(
            state.apply(&AdminCommand::Pause),
            Err(AdminError::AlreadyPaused)
        );
        assert_eq!(state.apply(&AdminCommand::Resume), Ok(AdminAck::Resumed));
        assert!(!state.paused);
        assert_eq!(
            state.apply(&AdminCommand::Resume),
            Err(AdminError::NotPaused)
        );
    }

    #[test]
    fn kill_increments_counter() {
        let mut state = AdminState::default();
        let ack = state.apply(&AdminCommand::Kill(42)).expect("kill");
        assert_eq!(ack, AdminAck::Killed { pid: 42 });
        assert_eq!(state.kills, 1);
    }

    #[test]
    fn show_target_is_validated() {
        assert!(matches!(
            AdminCommand::parse("SHOW FOO"),
            Err(AdminError::UnknownShowTarget(_))
        ));
    }
}
