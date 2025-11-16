#![feature(map_try_insert, iter_advance_by)]
mod running_text;
mod text_iter;
mod text_source;
mod cmd;
#[cfg(feature = "mpd")]
mod mpd;

use std::{
    cell::UnsafeCell, collections::BTreeMap, ffi::OsString, fs, io::{self, Write}, thread::sleep, time::Duration
};
#[cfg(feature = "mpd")]
use std::{net::SocketAddr};

use anyhow::anyhow;
use clap::{
    arg, builder::{BoolValueParser, OsStringValueParser, StringValueParser, TypedValueParser}, command, crate_description, crate_name, parser::ValueSource, value_parser, ArgAction, ArgGroup, ArgMatches, Command, Id
};
use text_source::TextSource;

use crate::{cmd::CmdSource, running_text::{RunIter, RunningText}, text_iter::TextIter};

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
    Right(bool),
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

fn source_from_token<'a, T>(token: &SourceToken, tokens: T, _args: SourceArgs) -> anyhow::Result<Box<dyn TextSource>>
where T: Iterator<Item = &'a ArgToken> {
    Ok(match token {
        SourceToken::String(s) => Box::new(s.to_owned()),
        SourceToken::File(f) => Box::new(fs::read_to_string(f)?),
        SourceToken::CmdArg(_) => Box::new(CmdSource::new(tokens
            .filter_map(|t| match t {
                ArgToken::Source(SourceToken::CmdArg(a)) => Some(a),
                _ => None,
            }))),
        SourceToken::Stdin => Box::new(io::read_to_string(io::stdin())?),
        #[cfg(feature = "mpd")]
        SourceToken::Mpd(addr) => Box::new(MpdSource::from_args(*addr, _args.mpd)?),
    })
}

fn text_from_matches(matches: &mut ArgMatches) -> anyhow::Result<Vec<TextIter>> {
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
    let mut right = false;

    let mut result = vec![];
    let mut current_args = None;
    let mut previous: Option<(&SourceToken, _, SourceArgs)> = None;
    for mut tokens in positional.values_mut().map(Iterator::peekable) {
        match tokens.peek().unwrap() {
            ArgToken::Source(source_token) => {
                if let Some((source_token, tokens, args)) = previous {
                    let new_source = source_from_token(source_token, tokens, args)?;
                    let mut new_replacements = replacements.clone();
                    new_replacements.push(("\n".to_owned(), newline.to_owned()));
                    result.push(TextIter::new(
                        new_source,
                        window as usize,
                        repeat,
                        separator.clone(),
                        new_replacements,
                        right,
                    ));
                    window = if let SourceToken::String(s) = source_token {
                        s.chars().count()
                    } else {
                        32
                    } as u64;
                    separator = &separator_default;
                    newline = &newline_default;
                    replacements = &replacements_default;
                    repeat = false;
                    right = false;
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
            ArgToken::Right(r) => {
                right = *r;
            },
        };
    }
    if let Some((source_token, tokens, args)) = previous {
        let new_source = source_from_token(source_token, tokens, args)?;
        let mut new_replacements = replacements.clone();
        new_replacements.push(("\n".to_owned(), newline.to_owned()));
        result.push(TextIter::new(
                new_source,
                window as usize,
                repeat,
                separator.clone(),
                new_replacements,
                right,
        ));
    }
    Ok(result)
}

fn main() -> anyhow::Result<()> {
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
        .arg(arg!(-R --right "Run text to the right")
            .value_parser(BoolValueParser::new()
                .map(ArgToken::Right))
            .num_args(0)
            .default_value("false")
            .default_missing_value("true")
            .action(ArgAction::Append))
        .arg(arg!(-e --replacements <REPLACE> "Key-value pairs of replacements. Specified as 'src=dest'.
Multiple replacements can be passed one argument separated by comma: -e src1=dest1,src2=dest2.
Useful for escaping special characters.")
             .value_parser(parse_key_value_pairs)
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
                &['\n' as u8]
            } else {
                &['\r' as u8]
            };

            let mut texts: Vec<UnsafeCell<RunningText>> = fragments
                .iter_mut()
                .map(|f| anyhow::Ok({
                    let content = f.source().get()?;
                    UnsafeCell::new(f.new_text(content))
                }))
                .collect::<anyhow::Result<_>>()?;

            let mut iters: Vec<RunIter<'_>> = texts
                .iter()
                .map(|r| unsafe { (&*r.get()).iter() })
                .collect();

            loop {
                for (i, it) in iters.iter_mut().enumerate() {
                    io::stdout().write(if fragments[i].right() {it.next_back()} else {it.next()}.unwrap().as_bytes())?;
                }
                io::stdout().write(line_terminator)?;
                io::stdout().flush()?;

                let changes: Vec<(usize, String)> = fragments
                    .iter_mut()
                    .enumerate()
                    .filter_map(|(i, f)| f.source().next().map(|t| anyhow::Ok((i, t?))))
                    .collect::<anyhow::Result<_>>()?;

                for (i, content) in changes {
                    let offset = iters[i].range().start;
                    texts[i] = fragments[i].new_text(content).into();
                    unsafe {
                        iters[i] = (&*texts[i].get()).iter_at(offset);
                    }
                }


                sleep(duration);
            }
        },
        _ => unreachable!(),
    }
}
