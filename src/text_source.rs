use clap::{ArgMatches, Id};

#[cfg(feature = "mpd")]
use std::net::SocketAddr;
use std::{
    fs::{self},
    io::{self},
    path::Path,
    process::Command,
};

#[cfg(feature = "mpd")]
use crate::mpd::{MpdFormatter, MpdSource, StateStatusIcons, StatusIcons, StatusIconsSet};

#[derive(Debug, Clone)]
pub struct Content {
    pub running: String,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug)]
pub struct ExecSource {
    pub cmd: Command,
    pub prefix: String,
    pub suffix: String,
    last_output: String,
}

impl ExecSource {
    pub fn get(&mut self, content: &mut String) -> bool {
        let output = String::from_utf8(
            self.cmd
                .spawn()
                .expect("Spawn of child failed")
                .wait_with_output()
                .expect("Waiting on child failed")
                .stdout,
        )
        .expect("Child's output is invalid UTF-8");
        if self.last_output == output {
            false
        } else {
            output.clone_into(content);
            true
        }
    }
}

#[derive(Debug)]
pub enum TextSource {
    String(Content),
    Exec(ExecSource),
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
            TextSource::Exec(s) => {
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
                c.get(
                    &mut content.running,
                    &mut content.prefix,
                    &mut content.suffix,
                )
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
    ) -> bool {
        match self {
            TextSource::String(_) => false,
            #[cfg(feature = "mpd")]
            TextSource::Mpd(s) => s.get(content, prefix, suffix).expect("MPD format error"),
            TextSource::Exec(s) => s.get(content),
        }
    }
    pub fn content_can_change(&self) -> bool {
        match self {
            Self::String(_) => false,
            Self::Exec(_) => false,
            #[cfg(feature = "mpd")]
            Self::Mpd(_) => true,
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
