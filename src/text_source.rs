use bitflags::bitflags;

use std::{
    ffi::{OsStr},
};

use crate::utils::Command;

#[cfg(feature = "mpd")]
use crate::mpd::{MpdSource};

#[derive(Debug, Clone)]
pub struct Content {
    pub running: String,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug)]
pub struct CmdSource {
    pub cmd: Command,
    pub prefix: String,
    pub suffix: String,
    last_output: String,
}

impl CmdSource {
    pub fn new<S: AsRef<OsStr>, I: IntoIterator<Item = S>>(
        args: I,
        prefix: String,
        suffix: String,
    ) -> Self {
        Self {
            cmd: args.into_iter().collect(),
            prefix,
            suffix,
            last_output: String::new(),
        }
    }
    pub fn get(&mut self, content: &mut String) -> anyhow::Result<ContentChange> {
        let output = self.cmd.spawn_and_read_output()?;
        if self.last_output == output {
            Ok(ContentChange::empty())
        } else {
            content.clear();
            output.clone_into(content);
            self.last_output = output;
            Ok(ContentChange::Running)
        }
    }
}

bitflags! {
    pub struct ContentChange: u8 {
        const Running = 1;
        const Prefix = 1 << 1;
        const Suffix = 1 << 2;
    }
}

#[derive(Debug)]
pub enum TextSource {
    String(Content),
    Cmd(CmdSource),
    #[cfg(feature = "mpd")]
    Mpd(Box<MpdSource>),
}

impl TextSource {
    pub fn content(running: String, prefix: String, suffix: String) -> Self {
        Self::String(Content {
            running,
            prefix,
            suffix,
        })
    }
    pub fn get_initial_content(&mut self) -> anyhow::Result<Content> {
        match self {
            TextSource::String(c) => Ok(c.clone()),
            TextSource::Cmd(s) => {
                let mut output = String::new();
                s.get(&mut output)?;
                Ok(Content {
                    running: output,
                    prefix: s.prefix.clone(),
                    suffix: s.suffix.clone(),
                })
            }
            #[cfg(feature = "mpd")]
            TextSource::Mpd(c) => {
                let mut content = Content {
                    running: String::new(),
                    prefix: String::new(),
                    suffix: String::new(),
                };
                c.running_format()
                    .format_with_source(c, &mut content.running)?;
                c.prefix_format()
                    .format_with_source(c, &mut content.prefix)?;
                c.suffix_format()
                    .format_with_source(c, &mut content.suffix)?;
                Ok(content)
            }
        }
    }
    pub fn get_content(
        &mut self,
        content: &mut String,
        #[cfg(feature = "mpd")] prefix: &mut String,
        #[cfg(feature = "mpd")] suffix: &mut String,
    ) -> anyhow::Result<ContentChange> {
        match self {
            TextSource::String(_) => Ok(ContentChange::empty()),
            #[cfg(feature = "mpd")]
            TextSource::Mpd(s) => s.get(content, prefix, suffix),
            TextSource::Cmd(s) => s.get(content),
        }
    }
}
