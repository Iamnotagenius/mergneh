use bitflags::bitflags;
use clap::{ArgMatches, Id};

use std::{
    ffi::{OsStr, OsString},
    fs::{self},
    io::{self},
    path::Path,
};

use crate::utils::Command;

#[cfg(feature = "mpd")]
use crate::mpd::{MpdFormatter, MpdSource, StatusIconsSet};

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
    pub fn get(&mut self, content: &mut String) -> ContentChange {
        let output = self.cmd.spawn_and_read_output().expect("Child error");
        if self.last_output == output {
            ContentChange::empty()
        } else {
            content.clear();
            output.clone_into(content);
            self.last_output = output;
            ContentChange::Running
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
    pub fn content(running: String, prefix: String, suffix: String) -> TextSource {
        TextSource::String(Content {
            running,
            prefix,
            suffix,
        })
    }
    pub fn get_initial_content(&mut self) -> Content {
        match self {
            TextSource::String(c) => c.clone(),
            TextSource::Cmd(s) => {
                let mut output = String::new();
                s.get(&mut output);
                Content {
                    running: output,
                    prefix: s.prefix.clone(),
                    suffix: s.suffix.clone(),
                }
            }
            #[cfg(feature = "mpd")]
            TextSource::Mpd(c) => {
                let mut content = Content {
                    running: String::new(),
                    prefix: if c.prefix_format().is_constant() {
                        c.prefix_format().to_string()
                    } else {
                        String::new()
                    },
                    suffix: if c.suffix_format().is_constant() {
                        c.suffix_format().to_string()
                    } else {
                        String::new()
                    },
                };
                c.running_format()
                    .format_with_source(c, &mut content.running)
                    .expect("MPD format error");
                c.prefix_format()
                    .format_with_source(c, &mut content.prefix)
                    .expect("MPD format error");
                c.suffix_format()
                    .format_with_source(c, &mut content.suffix)
                    .expect("MPD format error");
                content
            }
        }
    }
    pub fn get_content(
        &mut self,
        content: &mut String,
        prefix: &mut String,
        suffix: &mut String,
    ) -> ContentChange {
        match self {
            TextSource::String(_) => ContentChange::empty(),
            #[cfg(feature = "mpd")]
            TextSource::Mpd(s) => s.get(content, prefix, suffix).expect("MPD format error"),
            TextSource::Cmd(s) => s.get(content),
        }
    }
}

impl TryFrom<&mut ArgMatches> for TextSource {
    type Error = io::Error;

    fn try_from(value: &mut ArgMatches) -> Result<Self, Self::Error> {
        let kind = value.remove_one::<Id>("sources").unwrap();
        let src = value.try_remove_one::<String>(kind.as_str());
        let prefix = value.remove_one::<String>("prefix").unwrap();
        let suffix = value.remove_one::<String>("suffix").unwrap();
        return Ok(match kind.as_str() {
            "SOURCE" => {
                TextSource::content(from_file_or_string(&src.unwrap().unwrap())?, prefix, suffix)
            }
            "file" => {
                TextSource::content(fs::read_to_string(src.unwrap().unwrap())?, prefix, suffix)
            }
            "string" => TextSource::content(src.unwrap().unwrap(), prefix, suffix),
            "stdin" => TextSource::content(io::read_to_string(io::stdin())?, prefix, suffix),
            "cmd" => TextSource::Cmd(CmdSource::new(
                value.remove_many::<OsString>(kind.as_str()).unwrap(),
                prefix,
                suffix,
            )),
            #[cfg(feature = "mpd")]
            "mpd" => TextSource::Mpd(Box::new(MpdSource::new(
                value.try_remove_one(kind.as_str()).unwrap().unwrap(),
                value.remove_one("format").unwrap(),
                value
                    .remove_one("prefix-format")
                    .unwrap_or(MpdFormatter::only_string(prefix)),
                value
                    .remove_one("suffix-format")
                    .unwrap_or(MpdFormatter::only_string(suffix)),
                StatusIconsSet::new(
                    value.remove_one("status-icons").unwrap(),
                    value.remove_one("consume-icons").unwrap(),
                    value.remove_one("random-icons").unwrap(),
                    value.remove_one("repeat-icons").unwrap(),
                    value.remove_one("single-icons").unwrap(),
                ),
                value.remove_one("default-placeholder").unwrap(),
            ))),
            _ => unreachable!(),
        });
    }
}

fn from_file_or_string(arg: &str) -> io::Result<String> {
    let path = Path::new(arg);
    Ok(if path.is_file() {
        fs::read_to_string(path)?
    } else {
        arg.to_owned()
    })
}
