mod running_text;
mod utils;
mod text_source;
#[cfg(feature = "mpd")]
mod mpd;

use std::{
    collections::BTreeMap, ffi::OsString, fs, io::{self, Write}, iter::repeat_with, path::PathBuf, time::Duration
};
#[cfg(feature = "mpd")]
use std::{net::SocketAddr};

use anyhow::anyhow;
use clap::{
    arg, builder::{BoolValueParser, OsStringValueParser, StringValueParser, TypedValueParser}, command, crate_description, crate_name, parser::ValueSource, value_parser, ArgAction, ArgGroup, ArgMatches, Command, Id, ValueHint
};
use text_source::TextSource;
use ticker::Ticker;

use crate::{running_text::RunningText, text_source::{CmdSource}};

#[cfg(feature = "mpd")]
use crate::mpd::{MpdArgToken, MpdSource, MpdSourceArgs};

fn parse_key_value_pairs(value: &str) -> anyhow::Result<ArgToken> {
    if value.is_empty() {
        return Ok(ArgToken::Replacements(vec![]));
    }
    value.split(',')
        .map(|kv| kv
            .split_once('=')
            .map(|(l, r)| (l.to_owned(), r.to_owned()))
            .ok_or(anyhow!("Key-value pair must have at least one '=' sign")))
        .collect::<Result<_, _>>()
        .map(ArgToken::Replacements)
}

#[derive(Debug, Clone)]
pub enum SourceToken {
    String(String),
    File(String),
    CmdArg(OsString),
    Stdin,
    #[cfg(feature = "mpd")]
    Mpd(SocketAddr),
}

#[derive(Debug, Clone)]
pub enum ArgToken {
    Source(SourceToken),
    SourceArg(SourceArgToken),
    Window(u64),
    Separator(String),
    Newline(String),
    Replacements(Vec<(String, String)>),
    Repeat(bool),
    Reset(bool),
}

#[derive(Debug, Clone)]
pub enum SourceArgToken {
    #[cfg(feature = "mpd")]
    Mpd(MpdArgToken),
}

#[derive(Debug, Clone, Default)]
pub struct SourceArgs {
    #[cfg(feature = "mpd")]
    mpd: MpdSourceArgs,
}

fn source_from_token<'a, T>(token: &SourceToken, tokens: T, _args: SourceArgs) -> anyhow::Result<TextSource>
where T: Iterator<Item = &'a ArgToken> {
    Ok(match token {
        SourceToken::String(s) => TextSource::content(s.to_owned()),
        SourceToken::File(f) => TextSource::content(fs::read_to_string(f)?),
        SourceToken::CmdArg(_) => TextSource::Cmd(CmdSource::new(tokens
                .filter_map(|t| match t {
                    ArgToken::Source(SourceToken::CmdArg(a)) => Some(a),
                    _ => None,
                }))),
        SourceToken::Stdin => TextSource::content(io::read_to_string(io::stdin())?),
        #[cfg(feature = "mpd")]
        SourceToken::Mpd(addr) => TextSource::Mpd(Box::new(MpdSource::from_args(*addr, _args.mpd)?)),
    })
}


fn text_from_matches(matches: &mut ArgMatches) -> anyhow::Result<Vec<RunningText>> {
    // Create sources iteratively, from tokens (easier to parse positional arguments)
    let mut positional = BTreeMap::new();
    matches.ids()
        .filter(|id| matches
            .value_source(id.as_str()).unwrap() == ValueSource::CommandLine && matches.try_get_many::<Id>(id.as_str())
            .is_err())
        .for_each(|id| matches
            .indices_of(id.as_str())
            .unwrap()
            .zip(matches
                .get_occurrences::<ArgToken>(id.as_str())
                .unwrap())
            .for_each(|(i, value)| {
                positional.insert(i, value);
            }));
    
    let separator_default = "".to_string();
    let newline_default = "".to_string();
    let replacements_default = vec![];

    let mut window = 32;
    let mut separator = &separator_default;
    let mut newline = &newline_default;
    let mut replacements = &replacements_default;
    let mut repeat = false;
    let mut reset = false;

    let mut result = vec![];
    let mut current_args = None;
    let mut previous: Option<(&SourceToken, _, SourceArgs)> = None;
    for mut tokens in positional.values_mut().map(Iterator::peekable) {
        match tokens.peek().unwrap() {
            ArgToken::Source(source_token) => {
                if let Some((source_token, tokens, args)) = previous {
                    let new_source = source_from_token(source_token, tokens, args)?;
                    result.push(RunningText::new(
                        new_source,
                        window as usize,
                        separator.clone(),
                        newline.clone(),
                        replacements.clone(),
                        repeat,
                        reset,
                    )?);
                    window = if let SourceToken::String(s) = source_token {
                        s.chars().count()
                    } else {
                        32
                    } as u64;
                    separator = &separator_default;
                    newline = &newline_default;
                    replacements = &replacements_default;
                    repeat = false;
                    reset = false;
                }
                previous = Some((source_token, tokens, SourceArgs::default()));
                current_args = previous.as_mut().map(|t| &mut t.2);
            },
            ArgToken::SourceArg(token) => {
                if let Some(ref mut _args) = current_args {
                    
                    match token {
                        #[cfg(feature = "mpd")]
                        SourceArgToken::Mpd(token) => {
                            _args.mpd.apply_token(token);
                        },
                        #[cfg(not(feature = "mpd"))]
                        _ => unreachable!(),
                    };
                }
            },
            ArgToken::Window(w) => {
                window = *w;
            },
            ArgToken::Separator(s) => {
                separator = s;
            },
            ArgToken::Newline(n) => {
                newline = n;
            },
            ArgToken::Replacements(items) => {
                replacements = items;
            },
            ArgToken::Repeat(r) => {
                repeat = *r;
            },
            ArgToken::Reset(r) => {
                reset = *r;
            },
        };
    }
    if let Some((source_token, tokens, args)) = previous {
        let new_source = source_from_token(source_token, tokens, args)?;
        result.push(RunningText::new(
            new_source,
            window as usize,
            separator.clone(),
            newline.clone(),
            replacements.clone(),
            repeat,
            reset,
        )?);
    }
    Ok(result)
}

fn main() -> anyhow::Result<()> {
    // TODO:
    // - [WIP] support for multiple running texts (like each one has its own source etc)
    //   need to delete prefix and suffix
    //   also should use one client for mpd over several sources (poll in other thread,
    //   asynchronoua)
    // - support for long texts (without reading whole content)
    // - --once option for run subcommand
    let cli = command!(crate_name!())
        .about(crate_description!())
        .arg(arg!(-w --window <WINDOW> "Window size (if the corresponding source is string, will be equal to its length)")
            .value_parser(value_parser!(u64)
                .range(1..)
                .map(ArgToken::Window))
            .default_value("32")
            .action(ArgAction::Append))
        .arg(arg!(-s --separator <SEP> "String to print between content")
            .value_parser(StringValueParser::new()
                .map(ArgToken::Separator))
            .default_value("")
            .action(ArgAction::Append))
        .arg(arg!(-n --newline [NL] "String to replace newlines with")
            .value_parser(StringValueParser::new()
                .map(ArgToken::Newline))
            .default_value("")
            .default_missing_value("")
            .action(ArgAction::Append))
        .arg(arg!(-r --repeat "Repeat contents if it fits in the window size")
            .value_parser(BoolValueParser::new()
                .map(ArgToken::Repeat))
            .num_args(0)
            .default_value("false")
            .default_missing_value("true")
            .action(ArgAction::Append))
        .arg(arg!(--"reset-on-change" "Reset text window on content change")
            .value_parser(BoolValueParser::new()
                .map(ArgToken::Reset))
            .num_args(0)
            .default_value("false")
            .default_missing_value("true")
            .action(ArgAction::Append))
        .arg(arg!(-e --replacements <REPLACE> "Key-value pairs of replacements. Specified as 'src=dest'.
Multiple replacements can be passed either as one argument separated by comma: -e src1=dest1,src2=dest2
or as separated arguments: -e src1=dest1 -e src2=dest2.
Order of replacements matters. Useful for escaping special characters.")
             .value_parser(parse_key_value_pairs)
             .default_value("")
             .action(ArgAction::Append))
        .next_help_heading("Sources")
        .group(
            ArgGroup::new("sources")
            .required(true)
            .args(["file", "string", "stdin", "cmd"])
            .multiple(true),
        )
        .arg(arg!(-f --file <FILE> "Pull contents from a file (BEWARE: it loads whole file into memory!)")
            .value_parser(StringValueParser::new()
                .map(|s| ArgToken::Source(SourceToken::File(s))))
            .action(ArgAction::Append))
        .arg(arg!(-S --string <STRING> "Use a string as contents")
            .value_parser(StringValueParser::new()
                .map(|s| ArgToken::Source(SourceToken::String(s))))
            .action(ArgAction::Append))
        .arg(arg!(--stdin "Pull contents from stdin (BEWARE: it loads whole input into memory just like --file)")
            .value_parser(BoolValueParser::new()
                .map(|_| ArgToken::Source(SourceToken::Stdin)))
            .action(ArgAction::SetTrue))
        .arg(arg!(--cmd <ARGS> ... "Execute a command and use its output as contents (use a ';' as a terminator)")
             .value_parser(OsStringValueParser::new()
                 .map(|s| ArgToken::Source(SourceToken::CmdArg(s))))
             .num_args(1..)
             .value_terminator(";")
             .action(ArgAction::Append))
        .subcommand_required(true)
        .subcommand(
            Command::new("run")
                .arg(arg!(-d --duration <DURATION> "Tick duration")
                     .value_parser(value_parser!(humantime::Duration))
                     .default_value("1s"))
                .arg(arg!(-n --newline "Print each iteration on next line"))
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
    #[cfg(feature = "mpd")] 
    let cli = cli
        .arg(
            arg!(--mpd [SERVER_ADDR] "Display MPD status as running text [default server address is 127.0.0.0:6600]")
            .group("sources")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::Source(SourceToken::Mpd(s.parse()?))))
            .default_missing_value("127.0.0.0:6600")
            .action(ArgAction::Append)
        )
        .next_help_heading("MPD Options")
        .arg(
            arg!(--"status-icons" <ICONS> "Status icons to use")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::StateIcons(s.parse()?)))))
            .default_value("")
            .requires("mpd")
            .action(ArgAction::Append)
        )
        .arg(
            arg!(--"repeat-icons" <ICONS> "Repeat icons to use")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::RepeatIcons(s.parse()?)))))
            .default_value("")
            .requires("mpd")
            .action(ArgAction::Append)
        )
        .arg(
            arg!(--"consume-icons" <ICONS> "Consume icons to use")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::ConsumeIcons(s.parse()?)))))
            .default_value("")
            .requires("mpd")
            .action(ArgAction::Append)
        ) 
        .arg(
            arg!(--"random-icons" <ICONS> "Random icons to use")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::RandomIcons(s.parse()?)))))
            .default_value("")
            .requires("mpd")
            .action(ArgAction::Append)
        ) 
        .arg(
            arg!(--"single-icons" <ICONS> "Single icons to use")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::SingleIcons(s.parse()?)))))
            .default_value("")
            .requires("mpd")
            .action(ArgAction::Append)
        ) 
        .arg(
            arg!(--format <FORMAT> "Format string to use in running text")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::Format(s.parse()?)))))
            .default_value("{artist} - {title}")
            .requires("mpd")
            .action(ArgAction::Append)
        )
        .arg(
            arg!(-D --"default-placeholder" <PLACEHOLDER> "Default placeholder for missing values")
            .value_parser(|s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::Placeholder(s.to_owned())))))
            .default_value("N/A")
            .requires("mpd")
            .action(ArgAction::Append)
        );

    let mut matches = cli.get_matches();
    let mut fragments = text_from_matches(&mut matches)?;
    let (cmd, mut sub_matches) = matches.remove_subcommand().unwrap();
    match cmd.as_str() {
        "run" => {
            let duration: Duration = sub_matches
                .remove_one::<humantime::Duration>("duration")
                .unwrap().into();
            let line_terminator = if sub_matches.remove_one("newline").unwrap() {
                '\n'
            } else {
               '\r' 
            };
            let tick = Ticker::new(
                repeat_with(|| fragments
                    .iter_mut()
                    .map(|f| f.next().unwrap())
                    
                    .fold(Ok(String::new()), |s, r| match (s, r) {
                        (Ok(mut s), Ok(f)) => Ok({s.push_str(f.as_str()); s}),
                        (Ok(_), Err(e)) => Err(e),
                        (Err(e), _) => Err(e)
                    })
                ),
                duration);
            for text in tick {
                let mut text = text?;
                text.push(line_terminator);
                io::stdout().write(text.as_bytes())?;
                io::stdout().flush()?;
            }
        }
        "iter" => {
            let iter_file = sub_matches.remove_one::<PathBuf>("ITER_FILE").unwrap();
            let (_i, _prev_content) = match fs::read_to_string(&iter_file) {
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
            // let i = text.print_once(i, prev_content.as_str())?;
            // fs::write(iter_file, format!("{i} {}", text.get_raw_content()))?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
