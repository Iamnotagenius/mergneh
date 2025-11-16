use std::{
    error::Error,
    ffi::OsStr,
    fmt::Display,
    io,
    process::{self, Child, Stdio},
    string::FromUtf8Error,
};

use crate::text_source::TextSource;

#[derive(Debug)]
pub struct Command(process::Command);

#[derive(Debug)]
pub enum CommandError {
    Io(io::Error),
    UTF8(FromUtf8Error),
}

impl Error for CommandError {}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::Io(e) => write!(f, "Io error while executing command: {}", e),
            CommandError::UTF8(e) => write!(f, "Child process has outputed invalid UTF-8: {}", e),
        }
    }
}

impl Command {
    pub fn spawn_and_read_output(&mut self) -> Result<String, CommandError> {
        String::from_utf8(
            self.0
                .spawn()
                .and_then(Child::wait_with_output)
                .map_err(CommandError::Io)?
                .stdout,
        )
        .map_err(CommandError::UTF8)
    }
}

impl<S: AsRef<OsStr>> FromIterator<S> for Command {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let mut cmd = process::Command::new(
            iter.next()
                .expect("Iterator for Command must have at least one element"),
        );
        cmd.stdout(Stdio::piped()).args(iter);
        Command(cmd)
    }
}

impl From<Command> for process::Command {
    fn from(val: Command) -> Self {
        val.0
    }
}

impl From<process::Command> for Command {
    fn from(value: process::Command) -> Self {
        Command(value)
    }
}

#[derive(Debug)]
pub struct CmdSource {
    pub cmd: Command,
    last_output: String,
}

impl CmdSource {
    pub fn new<S: AsRef<OsStr>, I: IntoIterator<Item = S>>(
        args: I,
    ) -> Self {
        Self {
            cmd: args.into_iter().collect(),
            last_output: String::new(),
        }
    }
}

impl TextSource for CmdSource {
    fn get(&mut self) -> anyhow::Result<String> {
        if !self.last_output.is_empty() {
            Ok(self.last_output.clone())
        } else {
            self.get_if_changed().unwrap_or_else(|| Ok(String::new()))
        }
    }
    fn get_if_changed(&mut self) -> Option<anyhow::Result<String>> {
        let output = self.cmd.spawn_and_read_output();
        if let Err(e) = output {
            return Some(Err(e.into()));
        }

        let output = output.unwrap();
        if self.last_output == output {
            None
        } else {
            output.clone_into(&mut self.last_output);
            Some(Ok(output))
        }
    }
}
