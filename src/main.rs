mod running_text;
mod utils;

use std::{
    convert::Infallible,
    fs::File,
    io::{self, Read},
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use clap::{
    arg, builder::Str, command, crate_description, crate_name, ArgGroup, ArgMatches, Command, Id,
};

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
        let src = value.try_remove_one::<String>(kind.as_str());
        return Ok(match kind.as_str() {
            "SOURCE" => TextSource::from_str(&src.unwrap().unwrap()).unwrap(),
            "file" => TextSource::File(Arc::new(File::open(src.unwrap().unwrap())?)),
            "string" => TextSource::String(src.unwrap().unwrap()),
            "stdin" => TextSource::Stdin,
            _ => unreachable!(),
        });
    }
}

fn command_with_text_args(name: impl Into<Str>) -> Command {
    Command::new(name)
            .arg(arg!(<SOURCE> "same as --file, if file with this name does not exist or is a directory, it will behave as --string"))
            .arg(arg!(-f --file <FILE> "Pull contents from a file (BEWARE: it loads whole file into memory!)"))
            .arg(arg!(-S --string <STRING> "Use a string as contents"))
            .arg(arg!(--stdin "Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)"))
            .group(
                ArgGroup::new("sources")
                .required(true)
                .args(["SOURCE", "file", "string", "stdin"]),
                )
            .arg(arg!(-w --window <WINDOW> "Window size").default_value("6"))
            .arg(arg!(-s --separator <SEP> "String to print between content").default_value(""))
            .arg(arg!(-n --newline <NL> "String to replace newlines with").default_value(""))
            .arg(arg!(-l --prefix <PREFIX> "String to print before running text").default_value(""))
            .arg(arg!(-r --suffix <SUFFIX> "String to print after running text").default_value(""))
}

fn text_from_matches(mut matches: ArgMatches) -> Result<RunningText, io::Error> {
    let source = TextSource::try_from(&mut matches)?;
    let window_size = matches
        .remove_one::<String>("window")
        .map(|s| s.parse::<usize>().expect("Window size must be a number"))
        .unwrap();
    let separator = matches.remove_one::<String>("separator").unwrap();
    let newline = matches.remove_one::<String>("newline").unwrap();
    let prefix = matches.remove_one::<String>("prefix").unwrap();
    let suffix = matches.remove_one::<String>("suffix").unwrap();
    Ok(RunningText::new(
        source,
        window_size,
        separator,
        newline,
        prefix,
        suffix,
    )?)
}

fn main() -> Result<(), io::Error> {
    let (cmd, mut matches) = command!(crate_name!())
        .about(crate_description!())
        .subcommand_required(true)
        .subcommand(
            command_with_text_args("run")
                .arg(arg!(-d --duration <DURATION> "Tick duration").default_value("1s"))
                .about("Run text in a terminal")
                .arg_required_else_help(true),
        )
        .get_matches()
        .remove_subcommand()
        .unwrap();
    match cmd.as_str() {
        "run" => {
            let duration: Duration = matches
                .remove_one::<String>("duration")
                .map(|s| {
                    s.parse::<humantime::Duration>()
                        .expect("Duration parse error")
                        .into()
                })
                .unwrap();
            text_from_matches(matches)?.run_on_terminal(duration)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
