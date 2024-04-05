use std::{
    convert::Infallible,
    fs::File,
    io::{self, Read, Write},
    os::fd::AsFd,
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::{arg, command, ArgGroup, ArgMatches, Id};
use ticker::Ticker;

struct RunningText {
    source: String,
    duration: Duration,
    window_size: usize,
}

#[derive(Debug, Clone)]
enum TextSource {
    String(Arc<str>),
    File(Arc<File>),
}

impl FromStr for TextSource {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = Path::new(s);
        if path.is_dir() {
            return Ok(TextSource::String(Arc::from(s)));
        }
        return Ok(match File::open(path) {
            Ok(file) => TextSource::File(Arc::new(file)),
            Err(_) => TextSource::String(Arc::from(s)),
        });
    }
}

impl TextSource {
    fn to_string(self) -> Result<String, io::Error> {
        Ok(match self {
            TextSource::String(s) => s.to_string(),
            TextSource::File(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s);
                s
            }
        })
    }
}

impl TryFrom<&ArgMatches> for TextSource {
    type Error = io::Error;

    fn try_from(value: &ArgMatches) -> Result<Self, Self::Error> {
        let kind = value.get_one::<Id>("sources").unwrap().as_str();
        let src = value.get_one::<String>(kind).unwrap();
        return Ok(match kind {
            "SOURCE" => TextSource::from_str(src).unwrap(),
            "file" => TextSource::File(Arc::new(File::open(src)?)),
            "string" => TextSource::String(Arc::from(src.as_str())),
            _ => unreachable!(),
        });
    }
}

impl TextSource {
    fn len(&self) -> usize {
        match self {
            TextSource::String(s) => s.len(),
            TextSource::File(f) => f.metadata().unwrap().len() as usize,
        }
    }
}

impl RunningText {
    fn new(source: TextSource, duration: Duration, window_size: usize) -> Result<Self, io::Error> {
        Ok(RunningText {
            source: source.to_string()?,
            duration,
            window_size,
        })
    }
    fn run_on_console(self) -> Result<(), io::Error> {
        let tick = Ticker::new(0..self.source.len() - self.window_size + 1, self.duration);
        println!("Source: {}", self.source);
        print!("\r{}", &self.source[0..self.window_size]);
        for t in tick {
            print!("\r{}", &self.source[t..t + self.window_size]);
            io::stdout().flush()?;
        }
        return Ok(());
    }
}

fn main() -> Result<(), io::Error> {
    let mut matches = command!()
        .arg(arg!(<SOURCE> "File/string source"))
        .arg(arg!(-f --file <FILE> "File source"))
        .arg(arg!(-s --string <STRING> "String source"))
        .group(
            ArgGroup::new("sources")
                .required(true)
                .args(["SOURCE", "file", "string"]),
        )
        .arg(arg!(-d --duration <DURATION> "Tick duration"))
        .arg(arg!(-w --window <WINDOW> "Window size"))
        .get_matches();

    let source = TextSource::try_from(&matches)?;
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
