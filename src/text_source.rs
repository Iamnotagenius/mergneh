use clap::{ArgMatches, Id};
#[cfg(feature = "mpd")]
use mpd::Client;

use std::{path::Path, io::{self}, fs::{self}, net::SocketAddr};

#[derive(Debug)]
pub enum TextSource {
    String(String),
    #[cfg(feature = "mpd")]
    Mpd(Client),
}

impl TextSource {
    pub fn from_file_or_string(arg: &str) -> io::Result<TextSource> {
        let path = Path::new(arg);
        Ok(TextSource::String(if path.is_file() {
            fs::read_to_string(path)?
        } else {
            arg.to_owned()
        }))
    }
    pub fn get_content(&mut self) -> String {
        match self {
            TextSource::String(s) => s.clone(),
            #[cfg(feature = "mpd")]
            TextSource::Mpd(_) => todo!(),
        }
    }
    pub fn content_can_change(&self) -> bool {
        match self {
            Self::String(_) => false,
            #[cfg(feature = "mpd")]
            Self::Mpd(_) => true
        }
    }
}

impl TryFrom<&mut ArgMatches> for TextSource {
    type Error = io::Error;

    fn try_from(value: &mut ArgMatches) -> Result<Self, Self::Error> {
        let kind = value.remove_one::<Id>("sources").unwrap();
        let src = value.try_remove_one::<String>(kind.as_str());
        return Ok(match kind.as_str() {
            "SOURCE" => TextSource::from_file_or_string(&src.unwrap().unwrap()).unwrap(),
            "file" => TextSource::String(fs::read_to_string(src.unwrap().unwrap())?),
            "string" => TextSource::String(src.unwrap().unwrap()),
            "stdin" => TextSource::String(io::read_to_string(io::stdin())?),
            #[cfg(feature = "mpd")]
            "mpd" => TextSource::Mpd(Client::connect(value.try_remove_one::<SocketAddr>(kind.as_str()).unwrap().unwrap()).expect("MPD connection error")),
            _ => unreachable!(),
        });
    }
}

