mod running_text;

use std::{
    convert::Infallible,
    fs::File,
    io::{self, Read},
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::{arg, command, ArgGroup, ArgMatches, Id};

use crate::running_text::RunningText;

#[derive(Debug, Default, Clone)]
pub enum TextSource {
    String(String),
    File(Arc<File>),
    #[default]
    Stdin,
}

impl FromStr for TextSource {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = Path::new(s);
        if path.is_dir() {
            return Ok(TextSource::String(s.to_owned()));
        }
        return Ok(match File::open(path) {
            Ok(file) => TextSource::File(Arc::new(file)),
            Err(_) => TextSource::String(s.to_owned()),
        });
    }
}

impl TryInto<String> for TextSource {
    type Error = io::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        Ok(match self {
            TextSource::String(s) => s.to_string(),
            TextSource::File(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s)?;
                s
            }
            TextSource::Stdin => {
                let mut s = String::new();
                io::stdin().read_to_string(&mut s)?;
                s
            }
        })
    }
}

impl TryFrom<&mut ArgMatches> for TextSource {
    type Error = io::Error;

    fn try_from(value: &mut ArgMatches) -> Result<Self, Self::Error> {
        let kind = value.remove_one::<Id>("sources").unwrap();
        let src = value.remove_one::<String>(kind.as_str()).unwrap();
        return Ok(match kind.as_str() {
            "SOURCE" => TextSource::from_str(&src).unwrap(),
            "file" => TextSource::File(Arc::new(File::open(src)?)),
            "string" => TextSource::String(src),
            "stdin" => TextSource::Stdin,
            _ => unreachable!(),
        });
    }
}

fn main() -> Result<(), io::Error> {
    let mut matches = command!()
        .arg(arg!(<SOURCE> "File/string source"))
        .arg(arg!(-f --file <FILE> "File source"))
        .arg(arg!(-s --string <STRING> "String source"))
        .arg(arg!(--stdin "Read text from stdin"))
        .group(
            ArgGroup::new("sources")
                .required(true)
                .args(["SOURCE", "file", "string", "stdin"]),
        )
        .arg(arg!(-d --duration <DURATION> "Tick duration"))
        .arg(arg!(-w --window <WINDOW> "Window size"))
        .get_matches();

    let source = TextSource::try_from(&mut matches)?;
    let duration: Duration = matches
        .remove_one::<String>("duration")
        .map(|s| {
            s.parse::<humantime::Duration>()
                .expect("Duration parse error")
                .into()
        })
        .unwrap_or(Duration::from_secs(1));
    let window_size = matches
        .get_one::<String>("window")
        .map(|s| s.parse::<usize>().expect("Window size must be a number"))
        .unwrap_or(3);
    println!("Args: {source:?}, Duration: {duration:?}");
    RunningText::new(source, duration, window_size)?.run_on_console()?;
    Ok(())
}
