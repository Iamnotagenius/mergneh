mod running_text;
mod utils;
mod text_source;
#[cfg(feature = "mpd")]
mod mpd;
#[cfg(feature = "waybar")]
mod waybar;

use std::{
    fs::{self},
    io::{self},
    path::PathBuf,
    time::Duration, ffi::OsString,
};
#[cfg(feature = "mpd")]
use std::net::SocketAddr;

use clap::{
    arg, command, crate_description, crate_name, value_parser, ArgAction, ArgGroup, ArgMatches, Command, ValueHint
};
use text_source::TextSource;
#[cfg(feature = "waybar")]
use waybar::Tooltip;

use crate::running_text::RunningText;

#[cfg(feature = "mpd")]
use crate::mpd::{StatusIcons, StateStatusIcons, MpdFormatter};

fn text_from_matches(matches: &mut ArgMatches) -> anyhow::Result<RunningText> {
    RunningText::new(
        TextSource::try_from(&mut *matches)?,
        matches.remove_one::<u64>("window").unwrap() as usize,
        matches.remove_one("separator").unwrap(),
        matches.remove_one("newline").unwrap(),
        matches.remove_one("dont-repeat").unwrap(),
        matches.remove_one("reset-on-change").unwrap(),
    )
}

fn main() -> anyhow::Result<()> {
    let mut cli = command!(crate_name!())
        .about(crate_description!())
        .arg(arg!(-w --window <WINDOW> "Window size").value_parser(value_parser!(u64).range(1..)).default_value("32"))
        .arg(arg!(-s --separator <SEP> "String to print between content").default_value(""))
        .arg(arg!(-n --newline <NL> "String to replace newlines with").default_value(""))
        .arg(arg!(-l --prefix <PREFIX> "String to print before running text").default_value(""))
        .arg(arg!(-r --suffix <SUFFIX> "String to print after running text").default_value(""))
        .arg(arg!(-'1' --"dont-repeat" "Do not repeat contents if it fits in the window size").action(ArgAction::SetFalse))
        .arg(arg!(--"reset-on-change" "Reset text window on content change"))
        .next_help_heading("Sources")
        .arg(arg!(<SOURCE> "    Same as --file, if file with this name does not exist or is a directory, it will behave as --string"))
        .arg(arg!(-f --file <FILE> "Pull contents from a file (BEWARE: it loads whole file into memory!)"))
        .arg(arg!(-S --string <STRING> "Use a string as contents"))
        .arg(arg!(--stdin "Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)"))
        .arg(arg!(--cmd <ARGS> ... "Execute a command and use its output as contents (use a ';' as a terminator)")
             .value_parser(value_parser!(OsString))
             .num_args(1..)
             .value_terminator(";"))
        .group(
            ArgGroup::new("sources")
            .required(true)
            .args(["SOURCE", "file", "string", "stdin", "cmd"]),
            )
        .subcommand_required(true)
        .subcommand(
            Command::new("run")
                .arg(arg!(-d --duration <DURATION> "Tick duration")
                     .value_parser(value_parser!(humantime::Duration))
                     .default_value("1s"))
                .about("Run text in a terminal")
        )
        .subcommand(
            Command::new("iter")
                .arg(arg!(<ITER_FILE> "File containing data for next iteration")
                     .value_parser(value_parser!(PathBuf))
                     .value_hint(ValueHint::FilePath))
                .about("Print just one iteration")
                .arg_required_else_help(true),
        );
    #[cfg(feature = "waybar")] {
        let mut cmd = Command::new("waybar")
            .arg(arg!(-d --duration <DURATION> "Tick duration")
                 .value_parser(value_parser!(humantime::Duration))
                 .default_value("1s"))
            .arg(arg!([TOOLTIP] "Tooltip to show on hover"))
            .arg(arg!(--"tooltip-cmd" <ARGS> ... "Use output of a command for tooltip")
                 .value_parser(value_parser!(OsString))
                 .num_args(1..))
            .group(ArgGroup::new("tooltips")
                   .multiple(false)
                   .args(["TOOLTIP", "tooltip-cmd"]))
            .about("Run text with custom module in waybar (JSON output)");
        #[cfg(feature = "mpd")] {
            cmd = cmd.arg(arg!(-t --"tooltip-format" [FORMAT] "Tooltip format to use with MPD")
                          .value_parser(value_parser!(MpdFormatter))
                          .default_missing_value("{artist} - {title}")
                          .group("tooltips"));
        }
        cli = cli.subcommand(cmd);
    }
    #[cfg(feature = "mpd")] {
        cli = cli
            .arg(
                arg!(--mpd [SERVER_ADDR] "Display MPD status as running text [default server address is 127.0.0.0:6600]")
                      .group("sources")
                      .value_parser(value_parser!(SocketAddr))
                      .default_missing_value("127.0.0.0:6600")
                    )
        .next_help_heading("MPD Options")
        .arg(
            arg!(--"status-icons" <ICONS> "Status icons to use")
                .value_parser(value_parser!(StateStatusIcons))
                .default_value(""),
        )
        .arg(
            arg!(--"repeat-icons" <ICONS> "Repeat icons to use")
                .value_parser(value_parser!(StatusIcons))
                .default_value("凌稜")
                .requires("mpd")
        )
        .arg(
            arg!(--"consume-icons" <ICONS> "Consume icons to use")
            .value_parser(value_parser!(StatusIcons))
            .default_value("")
            .requires("mpd")
        ) 
        .arg(
            arg!(--"random-icons" <ICONS> "Random icons to use")
            .value_parser(value_parser!(StatusIcons))
            .default_value("")
            .requires("mpd")
        ) 
        .arg(
            arg!(--"single-icons" <ICONS> "Single icons to use")
            .value_parser(value_parser!(StatusIcons))
            .default_value("")
            .requires("mpd")
        ) 
        .arg(
            arg!(--format <FORMAT> "Format string to use in running text")
                .value_parser(value_parser!(MpdFormatter))
                .default_value("{artist} - {title}")
                .requires("mpd")
        )
        .arg(
            arg!(-L --"prefix-format" <FORMAT> "Format string to use in prefix")
                .value_parser(value_parser!(MpdFormatter))
                .conflicts_with("prefix")
                .requires("mpd")
        )
        .arg(
            arg!(-R --"suffix-format" <FORMAT> "Format string to use in suffix")
                .value_parser(value_parser!(MpdFormatter))
                .conflicts_with("suffix")
                .requires("mpd")
        )
        .arg(
            arg!(-D --"default-placeholder" <PLACEHOLDER> "Default placeholder for missing values")
                .default_value("N/A")
                .requires("mpd")
        );
    }
    let mut matches = cli.get_matches();
    let mut text = text_from_matches(&mut matches)?;
    let (cmd, mut sub_matches) = matches.remove_subcommand().unwrap();
    match cmd.as_str() {
        "run" => {
            let duration: Duration = sub_matches
                .remove_one::<humantime::Duration>("duration")
                .unwrap().into();
            text.run_on_terminal(duration)?;
        }
        "iter" => {
            let iter_file = sub_matches.remove_one::<PathBuf>("ITER_FILE").unwrap();
            let (i, prev_content) = match fs::read_to_string(&iter_file) {
                Ok(s) => match s.split_once(' ') {
                    Some((number, content)) => (
                        number
                            .parse::<usize>()
                            .map_err(|e| anyhow::anyhow!(e).context("Failed parsing iter file"))?,
                        content.to_owned(),
                    ),
                    _ => Err(anyhow::anyhow!("Wrong iter file format, it should be '<i> <prev_content>").context("Failed parsing iter file"))?,
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => (0, String::new()),
                    _ => return Err(e.into()),
                },
            };
            let i = text.print_once(i, prev_content.as_str())?;
            fs::write(iter_file, format!("{i} {}", text.get_raw_content()))?;
        }
        #[cfg(feature = "waybar")]
        "waybar" => {
            let duration: Duration = sub_matches
                .remove_one::<humantime::Duration>("duration")
                .unwrap().into();
            #[cfg(feature = "mpd")] {
                let tooltip = sub_matches.remove_one::<MpdFormatter>("tooltip-format")
                    .map(Tooltip::Mpd)
                    .or(sub_matches.remove_one("TOOLTIP").map(Tooltip::Simple))
                    .or(sub_matches.remove_many::<OsString>("tooltip-cmd").map(|vs| Tooltip::Cmd(vs.collect())));
                text.run_in_waybar(duration, tooltip)?;
            }
            #[cfg(not(feature = "mpd"))] {
                let tooltip = sub_matches.remove_one("TOOLTIP").map(Tooltip::Simple)
                    .or(sub_matches.remove_many::<OsString>("tooltip-cmd").map(|vs| Tooltip::Cmd(vs.collect())));
                text.run_in_waybar(duration, tooltip)?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
