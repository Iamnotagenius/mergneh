mod running_text;
mod utils;
#[cfg(feature = "mpd")]
mod mpd;
mod text_source;

use std::{
    fs::{self},
    io::{self},
    path::PathBuf,
    time::Duration, net::SocketAddr,
};

use clap::{
    arg, command, crate_description, crate_name, ArgAction, ArgGroup, ArgMatches, Command, value_parser,
};
use text_source::TextSource;

use crate::running_text::RunningText;

fn text_from_matches(matches: &mut ArgMatches) -> Result<RunningText, io::Error> {
    RunningText::new(
        TextSource::try_from(&mut *matches)?,
        matches
            .remove_one::<String>("window")
            .map(|s| s.parse::<usize>().expect("Window size must be a number"))
            .unwrap(),
        matches.remove_one::<String>("separator").unwrap(),
        matches.remove_one::<String>("newline").unwrap(),
        matches.remove_one::<String>("prefix").unwrap(),
        matches.remove_one::<String>("suffix").unwrap(),
        matches.remove_one::<bool>("dont-repeat").unwrap(),
    )
}

fn main() -> Result<(), io::Error> {
    let mut cli = command!(crate_name!())
        .about(crate_description!())
        .arg(arg!(-w --window <WINDOW> "Window size").default_value("6"))
        .arg(arg!(-s --separator <SEP> "String to print between content").default_value(""))
        .arg(arg!(-n --newline <NL> "String to replace newlines with").default_value(""))
        .arg(arg!(-l --prefix <PREFIX> "String to print before running text").default_value(""))
        .arg(arg!(-r --suffix <SUFFIX> "String to print after running text").default_value(""))
        .arg(arg!(-'1' --"dont-repeat" "Do not repeat contents if it fits in the window size").action(ArgAction::SetFalse))
        .next_help_heading("Sources")
        .arg(arg!(<SOURCE> "    Same as --file, if file with this name does not exist or is a directory, it will behave as --string"))
        .arg(arg!(-f --file <FILE> "Pull contents from a file (BEWARE: it loads whole file into memory!)"))
        .arg(arg!(-S --string <STRING> "Use a string as contents"))
        .arg(arg!(--stdin "Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)"))
        .group(
            ArgGroup::new("sources")
            .required(true)
            .args(["SOURCE", "file", "string", "stdin"]),
            )
        .subcommand_required(true)
        .subcommand(
            Command::new("run")
                .arg(arg!(-d --duration <DURATION> "Tick duration").default_value("1s"))
                .about("Run text in a terminal")
        )
        .subcommand(
            Command::new("iter")
                .arg(arg!(<ITER_FILE> "File containing data for next iteration").value_parser(clap::value_parser!(PathBuf)))
                .about("Print just one iteration")
                .arg_required_else_help(true),
        );
    #[cfg(feature = "mpd")] {
        cli = cli.arg(arg!(--mpd [SERVER_ADDR] "Display MPD status as running text [default server address is 127.0.0.0:6600]")
                      .group("sources")
                      .value_parser(value_parser!(SocketAddr))
                      .default_missing_value("127.0.0.0:6600"))
        .next_help_heading("MPD Options")
        .arg(
            arg!(--"status-icons" <ICONS> "Status icons")
                .value_parser(mpd::parse_player_icons)
                .default_value(""),
        )
        .arg(
            arg!(--"repeat-icons" <ICONS> "Repeat icons to use")
                .value_parser(mpd::parse_status_icons)
                .default_value("凌稜"),
        )
        .group(
            ArgGroup::new("mpd-options")
                .requires("mpd")
                .multiple(true)
                .args(["status-icons", "repeat-icons"]),
        );
    }

    let mut matches = cli.get_matches();
    let mut text = text_from_matches(&mut matches)?;
    let (cmd, mut sub_matches) = matches.remove_subcommand().unwrap();
    match cmd.as_str() {
        "run" => {
            let duration: Duration = sub_matches
                .remove_one::<String>("duration")
                .map(|s| {
                    s.parse::<humantime::Duration>()
                        .expect("Duration parse error")
                        .into()
                })
                .unwrap();
            text.run_on_terminal(duration)?;
        }
        "iter" => {
            let iter_file = sub_matches.remove_one::<PathBuf>("ITER_FILE").unwrap();
            let (i, prev_content) = match fs::read_to_string(&iter_file) {
                Ok(s) => match s.split_once(' ') {
                    Some((number, content)) => (
                        number
                            .parse::<usize>()
                            .expect("First word in iter file should be a number"),
                        content.to_owned(),
                    ),
                    _ => panic!("Wrong iter file format, it should be '<i> <prev_content>"),
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => (0, String::new()),
                    _ => return Err(e),
                },
            };
            let i = text.print_once(i, prev_content.as_str());
            fs::write(iter_file, format!("{i} {}", text.get_raw_content()))?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
