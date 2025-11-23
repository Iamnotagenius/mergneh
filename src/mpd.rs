use std::{
    collections::{BTreeMap, HashMap}, error::Error, fmt::{self, Display, Write}, net::SocketAddr, num::ParseIntError, str::FromStr, sync::{Arc, Mutex, TryLockError}, thread, time::{Duration, Instant}
};

use anyhow::{anyhow, Context};
use chrono::{
    format::{Item, StrftimeItems},
    NaiveTime,
};
use clap::{arg, builder::ValueParserFactory, ArgAction, Command};
use mpd::{song::QueuePlace, Client, Idle, Song, State, Status, Subsystem};

use crate::{text_source::TextSource, ArgToken, SourceArgToken, SourceToken};

pub fn mpd_args(cli: Command) -> Command {
    cli
        .next_help_heading("Sources")
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
        )
}

// Used for initializing threads for MPD pollers
static ADDRS: Mutex<BTreeMap<SocketAddr, Arc<Mutex<MpdState>>>> = Mutex::new(BTreeMap::new());

#[derive(Debug)]
pub enum IconSetParseError<const N: usize> {
    NotEnoughChars,
    TooManyChars,
}
impl<const N: usize> Display for IconSetParseError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconSetParseError::NotEnoughChars => {
                write!(f, "Not enough characters (expected {})", N)
            }
            IconSetParseError::TooManyChars => write!(f, "Too many characters (expected {})", N),
        }
    }
}
impl<const N: usize> Error for IconSetParseError<N> {}

#[derive(Debug, Clone, Copy)]
pub struct StateStatusIcons {
    play: char,
    pause: char,
    stop: char,
}

impl ValueParserFactory for StateStatusIcons {
    type Parser = fn(&str) -> anyhow::Result<ArgToken>;

    fn value_parser() -> Self::Parser {
        |s: &str| anyhow::Ok(ArgToken::SourceArg(SourceArgToken::Mpd(MpdArgToken::StateIcons(s.parse()?))))
    }
}

impl StateStatusIcons {
    pub fn get_icon(&self, state: State) -> char {
        match state {
            State::Stop => self.stop,
            State::Play => self.play,
            State::Pause => self.pause,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StatusIcons {
    enabled: char,
    disabled: Option<char>,
}

impl StatusIcons {
    pub fn single(c: char) -> Self {
        Self { enabled: c, disabled: None }
    }
    pub fn get_icon(&self, state: bool) -> Option<char> {
        if state {
            Some(self.enabled)
        } else {
            self.disabled
        }
    }

    pub fn write<T: Write>(&self, state: bool, pad: usize, f: &mut T) -> std::fmt::Result {
        if let Some(c) = self.get_icon(state) {
            write!(f, "{}{}", c, " ".repeat(pad))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct StatusIconsSet {
    state: StateStatusIcons,
    consume: StatusIcons,
    random: StatusIcons,
    repeat: StatusIcons,
    single: StatusIcons,
}

impl StatusIconsSet {
    pub fn new(
        state_icons: StateStatusIcons,
        consume_icons: StatusIcons,
        random_icons: StatusIcons,
        repeat_icons: StatusIcons,
        single_icons: StatusIcons,
    ) -> Self {
        Self {
            state: state_icons,
            consume: consume_icons,
            random: random_icons,
            repeat: repeat_icons,
            single: single_icons,
        }
    }

    pub fn write_bool<T: Write>(&self, ph: &Placeholder, value: bool, f: &mut T) -> fmt::Result {
        match ph {
            Placeholder::ConsumeIcon(pad) => self.consume.write(value, *pad, f),
            Placeholder::RandomIcon(pad) => self.random.write(value, *pad, f),
            Placeholder::RepeatIcon(pad) => self.repeat.write(value, *pad, f),
            Placeholder::SingleIcon(pad) => self.single.write(value, *pad, f),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Placeholder {
    String(String),
    Artist,
    AlbumArtist,
    Album,
    Title,
    Filename,
    Date,
    TotalTime(Vec<Item<'static>>),
    ElapsedTime(Vec<Item<'static>>),
    Volume,
    SongPosition,
    QueueLength,
    StateIcon(usize),
    ConsumeIcon(usize),
    RandomIcon(usize),
    RepeatIcon(usize),
    SingleIcon(usize),
}

#[derive(Debug, PartialEq)]
pub enum PlaceholderValue<'a> {
    String(&'a str),
    OptionalString(Option<&'a str>),
    Volume(i8),
    OptionalElapsedDuration(Option<Duration>, &'a Vec<Item<'static>>),
    OptionalDuration(Option<Duration>, &'a Vec<Item<'static>>),
    OptionalQueuePlace(Option<QueuePlace>),
    Len(u32),
    Bool(bool),
    State(State, usize),
}

impl Placeholder {
    pub fn get<'a>(&'a self, song: Option<&'a Song>, status: &Status, last_state_update_time: Instant) -> PlaceholderValue<'a> {
        let mut tags: HashMap<&str, &str> = song
            .map(|s| {
                s.tags
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        match self {
            Placeholder::String(s) => PlaceholderValue::String(s),
            Placeholder::Artist => PlaceholderValue::OptionalString(
                song.map(|s| s.artist.as_deref()).unwrap_or_default(),
            ),
            Placeholder::AlbumArtist => {
                PlaceholderValue::OptionalString(tags.remove("AlbumArtist"))
            }
            Placeholder::Album => PlaceholderValue::OptionalString(tags.remove("Album")),
            Placeholder::Title => PlaceholderValue::OptionalString(
                song.map(|s| s.title.as_deref()).unwrap_or_default(),
            ),
            Placeholder::Filename => {
                PlaceholderValue::OptionalString(song.map(|s| s.file.as_str()))
            }
            Placeholder::Date => PlaceholderValue::OptionalString(tags.remove("Date")),
            Placeholder::Volume => PlaceholderValue::Volume(status.volume),
            Placeholder::ElapsedTime(fmt) => {
                PlaceholderValue::OptionalDuration(match status.state {
                    State::Stop => None,
                    State::Play => status.elapsed.map(|d| last_state_update_time.elapsed() + d),
                    State::Pause => status.elapsed,
                }, fmt)
            }
            Placeholder::TotalTime(fmt) => PlaceholderValue::OptionalDuration(status.duration, fmt),
            Placeholder::SongPosition => PlaceholderValue::OptionalQueuePlace(status.song),
            Placeholder::QueueLength => PlaceholderValue::Len(status.queue_len),
            Placeholder::StateIcon(pad) => PlaceholderValue::State(status.state, *pad),
            Placeholder::ConsumeIcon(_) => PlaceholderValue::Bool(status.consume),
            Placeholder::RandomIcon(_) => PlaceholderValue::Bool(status.random),
            Placeholder::RepeatIcon(_) => PlaceholderValue::Bool(status.repeat),
            Placeholder::SingleIcon(_) => PlaceholderValue::Bool(status.single),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MpdFormatter(Vec<Placeholder>);

#[derive(Debug)]
pub enum MpdFormatParseError {
    UnknownPlaceholder(String),
    RedundantFormat(String),
    DurationParseError(chrono::format::ParseError),
    PadParseError(ParseIntError),
    UnmatchedParenthesis,
}

impl Display for MpdFormatParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPlaceholder(placeholder) => {
                write!(f, "Unknown placeholder '{placeholder}'")
            }
            Self::RedundantFormat(placeholder) => {
                write!(f, "'{placeholder}' does not have additional formatting")
            }
            Self::DurationParseError(e) => {
                write!(f, "Invalid duration format: {e}")
            }
            Self::PadParseError(e) => write!(f, "Padding parse error: {e}"),
            Self::UnmatchedParenthesis => write!(f, "Unmatched '{{' or '}}"),
        }
    }
}
impl Error for MpdFormatParseError {}

#[derive(Debug, Clone)]
pub enum MpdArgToken {
    Format(MpdFormatter),
    Placeholder(String),
    StateIcons(StateStatusIcons),
    ConsumeIcons(StatusIcons),
    RandomIcons(StatusIcons),
    RepeatIcons(StatusIcons),
    SingleIcons(StatusIcons),
}

#[derive(Debug, Clone)]
pub struct MpdSourceArgs {
    fmt: MpdFormatter,
    default_placeholder: String,
    state_icons: StateStatusIcons,
    consume_icons: StatusIcons,
    random_icons: StatusIcons,
    repeat_icons: StatusIcons,
    single_icons: StatusIcons,
}

impl MpdSourceArgs {
    pub fn apply_token(&mut self, token: &MpdArgToken) {
        match token {
            MpdArgToken::Format(mpd_formatter) => {
                self.fmt = mpd_formatter.clone();
            },
            MpdArgToken::Placeholder(p) => {
                self.default_placeholder = p.to_owned();
            },
            MpdArgToken::StateIcons(state_status_icons) => {
                self.state_icons = *state_status_icons;
            },
            MpdArgToken::ConsumeIcons(status_icons) => {
                self.consume_icons = *status_icons;
            },
            MpdArgToken::RandomIcons(status_icons) => {
                self.random_icons = *status_icons;
            },
            MpdArgToken::RepeatIcons(status_icons) => {
                self.repeat_icons = *status_icons;
            },
            MpdArgToken::SingleIcons(status_icons) => {
                self.single_icons = *status_icons;
            },
        }
    }
}

impl Default for MpdSourceArgs {
    fn default() -> Self {
        Self {
            fmt: MpdFormatter(vec![
                Placeholder::Artist,
                Placeholder::String(" - ".to_owned()),
                Placeholder::Title,
            ]),
            default_placeholder: "N/A".to_owned(),
            state_icons: StateStatusIcons {
                play: '',
                pause: '',
                stop: '',
            },
            consume_icons: StatusIcons::single(''),
            random_icons: StatusIcons::single(''),
            repeat_icons: StatusIcons::single(''),
            single_icons: StatusIcons::single('S'),
        }
    }
}

#[derive(Debug)]
pub struct MpdState {
    song: Option<Song>,
    status: Status,
    update_time: Instant,
}

#[derive(Debug)]
pub struct MpdSource {
    state: Arc<Mutex<MpdState>>,
    last_state_update_time: Instant,
    format: MpdFormatter,
    icons: StatusIconsSet,
    default_placeholder: String,
}

impl MpdSource {
    pub fn from_args(addr: SocketAddr, args: MpdSourceArgs) -> anyhow::Result<Self> {
        Self::new(
            addr,
            args.fmt,
            StatusIconsSet {
                state: args.state_icons,
                consume: args.consume_icons,
                random: args.random_icons,
                repeat: args.repeat_icons,
                single: args.single_icons,
            },
            args.default_placeholder
        )
    }
    pub fn new(
        addr: SocketAddr,
        fmt: MpdFormatter,
        icons: StatusIconsSet,
        default_placeholder: String,
    ) -> anyhow::Result<Self> {
        let mut l = ADDRS.lock().unwrap();
        let state = match l.try_insert(addr, Arc::new(Mutex::new(MpdState {
            song: None,
            status: Status::default(),
            update_time: Instant::now(),
        }))) {
            Err(e) => {
                e.entry.get().clone()
            }
            Ok(s) => {
                let state = s.clone();
                let mut client = Client::connect(addr).context("MPD connection error")?;
                thread::spawn(move || {
                    let mut song = client.currentsong().expect("MPD connection error");
                    let mut status = client.status().expect("MPD connection error");
                    let mut update_time = Instant::now();
                    *state.lock().unwrap() = MpdState { song, status, update_time };
                    
                    loop {
                        client.wait(&[
                            Subsystem::Player,
                            Subsystem::Queue,
                            Subsystem::Options,
                            Subsystem::Mixer
                        ]).expect("MPD connection error");
                        song = client.currentsong().expect("MPD connection error");
                        status = client.status().expect("MPD connection error");
                        update_time = Instant::now();
                        *state.lock().unwrap() = MpdState { song, status, update_time };
                    };
                });
                s.clone()
            },
        };
        Ok(Self {
            state: state,
            last_state_update_time: Instant::now(),
            format: fmt,
            icons,
            default_placeholder,
        })
    }
}

impl TextSource for MpdSource {
    fn get(&mut self) -> anyhow::Result<String> {
        let lock = match self.state.lock() {
            Err(e) => {
                println!("another thread has panicked");
                e.into_inner()
            },
            Ok(l) => l, 
        };
        self.format.format(
            &self.icons,
            lock.song.as_ref(),
            &lock.status,
            lock.update_time,
            &self.default_placeholder,
        )
    }
    fn get_if_changed(&mut self) -> Option<anyhow::Result<String>> {
        let lock = match self.state.try_lock() {
            Err(TryLockError::Poisoned(l)) => return Some(Err(anyhow!(l.to_string()).context("another thread has panicked"))),
            Err(TryLockError::WouldBlock) => return None,
            Ok(l) => l, 
        };

        if lock.update_time == self.last_state_update_time &&
            !(self.format.iter().any(|ph| matches!(ph, Placeholder::ElapsedTime(_))) && lock.status.state == State::Play) {
            return None
        }

        self.last_state_update_time = lock.update_time;

        Some(self.format.format(
            &self.icons,
            lock.song.as_ref(),
            &lock.status,
            lock.update_time,
            &self.default_placeholder,
        ))
    }
}

impl MpdFormatter {
    pub fn only_string(str: String) -> Self {
        Self(vec![Placeholder::String(str)])
    }
    pub fn format_with_source(&self, source: &MpdSource) -> anyhow::Result<String> {
        let lock = source.state.lock().unwrap();
        self.format(
            &source.icons,
            lock.song.as_ref(),
            &lock.status,
            lock.update_time,
            &source.default_placeholder,
        )
    }

    pub fn format(
        &self,
        icons: &StatusIconsSet,
        song: Option<&Song>,
        status: &Status,
        last_state_update_time: Instant,
        default: &str,
    ) -> anyhow::Result<String> {
        let mut f = String::new();
        for ph in self.iter() {
            match ph.get(song, status, last_state_update_time) {
                PlaceholderValue::String(s) => write!(f, "{}", s)?,
                PlaceholderValue::OptionalString(s) => write!(f, "{}", s.unwrap_or(default))?,
                PlaceholderValue::Volume(v) => write!(f, "{}", v)?,
                PlaceholderValue::Len(l) => write!(f, "{}", l)?,
                PlaceholderValue::OptionalDuration(op, fmt) | PlaceholderValue::OptionalElapsedDuration(op, fmt) => match op {
                    Some(d) => write!(
                        f,
                        "{}",
                        chrono::format::DelayedFormat::new(
                            None,
                            NaiveTime::from_num_seconds_from_midnight_opt(
                                d.as_secs() as _,
                                d.subsec_nanos() as _
                            ),
                            fmt.iter()
                        )
                    )
                    .map_err(|e| anyhow::anyhow!(e).context("Unsupported time specifier"))?,
                    None => write!(f, "{}", default)?,
                },
                PlaceholderValue::OptionalQueuePlace(op) => match op {
                    Some(qp) => write!(f, "{}", qp.pos + 1),
                    None => write!(f, "{}", default),
                }?,
                PlaceholderValue::Bool(b) => icons.write_bool(ph, b, &mut f)?,
                PlaceholderValue::State(s, pad) => {
                    write!(f, "{}{}", icons.state.get_icon(s), " ".repeat(pad))?
                }
            };
        }
        Ok(f)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Placeholder> {
        self.0.iter()
    }
}

impl Display for MpdFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ph in self.iter() {
            if let Placeholder::String(s) = ph {
                for part in s.split_inclusive(['{', '}']) {
                    write!(f, "{}", part)?;
                    match part.chars().last().expect("Part must not be empty") {
                        c if matches!(c, '{' | '}') => write!(f, "{}", c)?,
                        _ => continue,
                    };
                }
            } else {
                write!(
                    f,
                    "{}",
                    match ph {
                        Placeholder::Album => "{album}",
                        Placeholder::AlbumArtist => "{albumArtist}",
                        Placeholder::Artist => "{artist}",
                        Placeholder::ConsumeIcon(_) => "{consumeIcon}",
                        Placeholder::Date => "{date}",
                        Placeholder::ElapsedTime(_) => "{elapsedTime}",
                        Placeholder::Filename => "{filename}",
                        Placeholder::QueueLength => "{queueLength}",
                        Placeholder::RandomIcon(_) => "{randomIcon}",
                        Placeholder::RepeatIcon(_) => "{repeatIcon}",
                        Placeholder::SingleIcon(_) => "{singleIcon}",
                        Placeholder::SongPosition => "{songPosition}",
                        Placeholder::StateIcon(_) => "{stateIcon}",
                        Placeholder::Title => "{title}",
                        Placeholder::TotalTime(_) => "{totalTime}",
                        Placeholder::Volume => "{volume}",
                        Placeholder::String(_) => unreachable!(),
                    }
                )?;
            }
        }
        Ok(())
    }
}

impl FromStr for MpdFormatter {
    type Err = MpdFormatParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut placeholders = Vec::new();
        let mut raw = String::new();
        let mut parse_slice = s;
        while !parse_slice.is_empty() {
            let left_par = match parse_slice.find(['{', '}']) {
                Some(i) => i,
                None => {
                    raw.push_str(parse_slice);
                    break;
                }
            };
            if let Some('}') = &parse_slice[left_par..].chars().next() {
                match parse_slice[left_par + 1..].chars().next() {
                    Some('}') => {
                        raw.push_str(&parse_slice[..left_par + 1]);
                        parse_slice = &parse_slice[left_par + 2..];
                        continue;
                    }
                    _ => return Err(MpdFormatParseError::UnmatchedParenthesis),
                };
            }

            if let Some('{') = &parse_slice[left_par + 1..].chars().next() {
                raw.push_str(&parse_slice[..left_par + 1]);
                parse_slice = &parse_slice[left_par + 2..];
                continue;
            }
            raw.push_str(&parse_slice[..left_par]);
            parse_slice = &parse_slice[left_par + 1..];
            if !raw.is_empty() {
                placeholders.push(Placeholder::String(raw));
                raw = String::new();
            }

            let right_par = match parse_slice.find(['{', '}']) {
                Some(i) => i,
                None => return Err(MpdFormatParseError::UnmatchedParenthesis),
            };
            if let Some('{') = parse_slice[right_par..].chars().next() {
                return Err(MpdFormatParseError::UnmatchedParenthesis);
            }
            let ph_spec = &parse_slice[..right_par];
            placeholders.push(if let Some((ph_type, ph_fmt)) = ph_spec.split_once(':') {
                match ph_type {
                    "date" => Placeholder::Date,
                    "elapsedTime" => Placeholder::ElapsedTime(
                        StrftimeItems::new(ph_fmt)
                            .parse_to_owned()
                            .map_err(MpdFormatParseError::DurationParseError)?,
                    ),
                    "totalTime" => Placeholder::TotalTime(
                        StrftimeItems::new(ph_fmt)
                            .parse_to_owned()
                            .map_err(MpdFormatParseError::DurationParseError)?,
                    ),
                    "consumeIcon" | "repeatIcon" | "stateIcon" | "singleIcon" | "randomIcon" => {
                        let pad = ph_fmt
                            .parse::<usize>()
                            .map_err(MpdFormatParseError::PadParseError)?;
                        match ph_type {
                            "consumeIcon" => Placeholder::ConsumeIcon(pad),
                            "repeatIcon" => Placeholder::RepeatIcon(pad),
                            "stateIcon" => Placeholder::StateIcon(pad),
                            "singleIcon" => Placeholder::SingleIcon(pad),
                            "randomIcon" => Placeholder::RandomIcon(pad),
                            _ => unreachable!(),
                        }
                    }
                    _ => return Err(MpdFormatParseError::RedundantFormat(ph_type.to_owned())),
                }
            } else {
                match ph_spec {
                    "album" => Placeholder::Album,
                    "albumArtist" => Placeholder::AlbumArtist,
                    "artist" => Placeholder::Artist,
                    "consumeIcon" => Placeholder::ConsumeIcon(0),
                    "date" => Placeholder::Date,
                    "elapsedTime" => Placeholder::ElapsedTime(
                        StrftimeItems::new("%M:%S").parse_to_owned().unwrap(),
                    ),
                    "filename" => Placeholder::Filename,
                    "queueLength" => Placeholder::QueueLength,
                    "randomIcon" => Placeholder::RandomIcon(0),
                    "repeatIcon" => Placeholder::RepeatIcon(0),
                    "singleIcon" => Placeholder::SingleIcon(0),
                    "songPosition" => Placeholder::SongPosition,
                    "stateIcon" => Placeholder::StateIcon(0),
                    "title" => Placeholder::Title,
                    "totalTime" => Placeholder::TotalTime(
                        StrftimeItems::new("%M:%S").parse_to_owned().unwrap(),
                    ),
                    "volume" => Placeholder::Volume,
                    _ => {
                        return Err(MpdFormatParseError::UnknownPlaceholder(
                            parse_slice[..right_par].to_owned(),
                        ))
                    }
                }
            });
            parse_slice = &parse_slice[right_par + 1..];
        }
        if !raw.is_empty() {
            placeholders.push(Placeholder::String(raw));
        }
        Ok(Self(placeholders))
    }
}

macro_rules! next_or_err {
    ($iter:ident => $type:ident: $($field:ident),+) => {
        $type {
            $($field: $iter.next().ok_or(IconSetParseError::NotEnoughChars)?),+
        }
    };
}

impl FromStr for StateStatusIcons {
    type Err = IconSetParseError<3>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.chars();
        let result = Ok(next_or_err!(iter => StateStatusIcons: play, pause, stop));
        if iter.next().is_some() {
            return Err(IconSetParseError::TooManyChars);
        }
        result
    }
}

impl FromStr for StatusIcons {
    type Err = IconSetParseError<2>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.chars();
        let result = Ok(StatusIcons {
            enabled: iter.next().ok_or(IconSetParseError::NotEnoughChars)?,
            disabled: iter.next(),
        });
        if iter.next().is_some() {
            return Err(IconSetParseError::TooManyChars);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::mpd::{MpdFormatParseError, MpdFormatter, Placeholder};
    use chrono::format::StrftimeItems;
    macro_rules! ph {
        ($p:ident) => {
            Placeholder::$p
        };
        ($p:ident(#$v:literal)) => {
            Placeholder::$p($v)
        };
        ($p:ident(*$v:literal)) => {
            Placeholder::$p(StrftimeItems::new($v).parse_to_owned().unwrap())
        };
        ($str:literal) => {
            Placeholder::String($str.to_owned())
        };
    }
    #[test]
    fn format_parse_test() {
        macro_rules! assert_ok {
            ($str:literal => [$($item:tt$(($h:tt$time:literal))?),*]) => {
                assert_eq!($str.parse::<MpdFormatter>().unwrap().0, vec![$(ph!($item$(($h$time))?)),*])
            };
        }
        macro_rules! assert_err {
            ($str:literal => $err:ident$(($s:literal))?) => {
                assert!(matches!($str.parse::<MpdFormatter>().unwrap_err(), MpdFormatParseError::$err$((s) if s.as_str() == $s)?));
            };
        }
        assert_ok!("rawstr" => ["rawstr"]);
        assert_ok!("" => []);
        assert_ok!("{artist} - {title}" => [Artist, " - ", Title]);
        assert_ok!(" [{elapsedTime}/{totalTime}] {stateIcon}" => [" [", ElapsedTime(*"%M:%S"), "/", TotalTime(*"%M:%S"), "] ", StateIcon(#0)]);
        assert_ok!(
            " [{elapsedTime:%M with %S}/{totalTime:%H hours %M minutes %S seconds}] {stateIcon:1}"
            => [" [", ElapsedTime(*"%M with %S"), "/", TotalTime(*"%H hours %M minutes %S seconds"), "] ", StateIcon(#1)]
        );
        assert_ok!("{{}}" => ["{}"]);
        assert_ok!("{{{artist}}}" => ["{", Artist, "}"]);
        assert_ok!("{{{artist}{title}}}" => ["{", Artist, Title, "}"]);
        assert_ok!("{artist} {title}}}" => [Artist, " ", Title, "}"]);
        assert_ok!("}}{{}}}}" => ["}{}}"]);
        assert_ok!("{{{artist}}}{title}" => ["{", Artist, "}", Title]);
        assert_ok!("{artist}{title}" => ["", Artist, "", Title, ""]);
        assert_ok!("}}{{{artist}}}{title}}}" => ["}{", Artist, "}", Title, "}"]);
        assert_err!("{artst}" => UnknownPlaceholder("artst"));
        assert_err!("{}artist}}" => UnknownPlaceholder(""));
        assert_err!("{ar}tst}" => UnknownPlaceholder("ar"));
        assert_err!("{artist}}" => UnmatchedParenthesis);
        assert_err!("}{artist}" => UnmatchedParenthesis);
        assert_err!("}}{{{artist}}}{title}}}" => UnknownPlaceholder("artist"));
        assert_err!("}}{{{artist}}}{{title}}}" => UnmatchedParenthesis);
        assert_err!("{{{{artist}}}" => UnmatchedParenthesis);
        assert_err!("{{{artist}}}{" => UnmatchedParenthesis);
        assert_err!("{{{artist}}}}" => UnmatchedParenthesis);
    }

    #[test]
    fn format_display_test() {
        macro_rules! assert {
            ([$($item:tt),*] => $str:literal) => {
                assert_eq!(MpdFormatter(vec![$(ph!($item)),*]).to_string(), $str)
            };
        }
        assert!([Artist, " - ", Title] => "{artist} - {title}");
        assert!([Artist, "{ - }", Title] => "{artist}{{ - }}{title}");
        assert!(["}", Artist, "{ -{ }", Title] => "}}{artist}{{ -{{ }}{title}");
        assert!([] => "");
    }

    #[test]
    fn format_back_and_forth_test() {
        macro_rules! assert {
            ($str:literal) => {
                assert_eq!($str.parse::<MpdFormatter>().unwrap().to_string(), $str)
            };
        }
        assert!("rawstr");
        assert!("");
        assert!("{artist} - {title}");
        assert!("{{}}");
        assert!("{{{artist}}}");
        assert!("{{{artist}{title}}}");
        assert!("{artist} {title}}}");
        assert!("}}{{}}}}");
        assert!("{{{artist}}}{title}");
        assert!("{artist}{title}");
        assert!("}}{{{artist}}}{title}}}");
    }
}
