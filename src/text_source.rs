use std::{
    ffi::{OsStr},
};

use crate::utils::Command;

#[cfg(feature = "mpd")]
use crate::mpd::{MpdSource};

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
    pub fn get(&mut self, content: &mut String) -> anyhow::Result<bool> {
        let output = self.cmd.spawn_and_read_output()?;
        if self.last_output == output {
            Ok(false)
        } else {
            content.clear();
            output.clone_into(content);
            self.last_output = output;
            Ok(true)
        }
    }
}

#[derive(Debug)]
pub enum TextSource {
    String(String),
    Cmd(CmdSource),
    #[cfg(feature = "mpd")]
    Mpd(Box<MpdSource>),
}

impl TextSource {
    pub fn content(running: String) -> Self {
        Self::String(running)
    }
    pub fn get_initial_content(&mut self) -> anyhow::Result<String> {
        match self {
            TextSource::String(c) => Ok(c.clone()),
            TextSource::Cmd(s) => {
                let mut output = String::new();
                s.get(&mut output)?;
                Ok(output)
            }
            #[cfg(feature = "mpd")]
            TextSource::Mpd(c) => {
                let mut content = String::new();
                c.format()
                    .format_with_source(c, &mut content)?;
                Ok(content)
            }
        }
    }
    pub fn get_content(
        &mut self,
        content: &mut String,
    ) -> anyhow::Result<bool> {
        match self {
            TextSource::String(_) => Ok(false),
            #[cfg(feature = "mpd")]
            TextSource::Mpd(s) => s.get(content),
            TextSource::Cmd(s) => s.get(content),
        }
    }
}
