use std::{
    error::Error,
    ffi::OsStr,
    fmt::Display,
    io,
    iter::repeat,
    process::{self, Child, Stdio},
    string::FromUtf8Error,
};

pub fn replace_newline(text: &mut String, replacement: &str) {
    text.retain(|c| c != '\r');
    if replacement.is_empty() {
        text.retain(|c| c != '\n');
        return;
    }
    let newline_count = text.chars().filter(|&c| c == '\n').count();
    let additional_len = (replacement.len() - 1) * newline_count;
    text.reserve(additional_len);
    text.extend(repeat('\0').take(additional_len));

    let mut dest = text.len();
    let mut src = text.len() - additional_len;

    unsafe {
        let buffer = text.as_bytes_mut();
        while src >= 1 {
            src -= 1;
            let byte = buffer[src];
            if byte == b'\n' {
                dest -= replacement.len();
                buffer[dest..dest + replacement.len()].copy_from_slice(replacement.as_bytes());
            } else {
                dest -= 1;
                buffer[dest] = byte;
            }
        }
    }
}

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
