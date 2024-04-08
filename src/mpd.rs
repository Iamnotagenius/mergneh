use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    fmt::{self, Write},
    net::SocketAddr,
    str::FromStr,
};

use mpd::{Client, Song, State, Status};

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

#[derive(Debug, Clone)]
pub struct StateStatusIcons {
    play: char,
    pause: char,
    stop: char,
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

#[derive(Debug, Clone)]
pub struct StatusIcons {
    enabled: char,
    disabled: Option<char>,
}

impl StatusIcons {
    pub fn get_icon(&self, state: bool) -> Option<char> {
        if state {
            Some(self.enabled)
        } else {
            self.disabled
        }
    }

    pub fn write<T: Write>(&self, state: bool, f: &mut T) -> std::fmt::Result {
        if let Some(c) = self.get_icon(state) {
            write!(f, "{}", c)
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
    Volume,
    ElapsedTime,
    TotalTime,
    SongPosition,
    QueueLength,
    StateIcon,
    ConsumeIcon,
    RandomIcon,
    RepeatIcon,
    SingleIcon,
}

#[derive(Debug, Clone, Default)]
pub struct MpdFormatter(Vec<Placeholder>);

#[derive(Debug)]
pub enum MpdFormatParseError {
    UnknownPlaceholder(String),
    UnmatchedParenthesis,
}

impl Display for MpdFormatParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MpdFormatParseError::UnknownPlaceholder(placeholder) => {
                write!(f, "Unknown placeholder '{placeholder}'")
            }
            MpdFormatParseError::UnmatchedParenthesis => write!(f, "Unmatched '{{' or '}}"),
        }
    }
}
impl Error for MpdFormatParseError {}

#[derive(Debug)]
pub struct MpdSource {
    client: Client,
    current_song: Option<Song>,
    current_status: Option<Status>,
    running_format: MpdFormatter,
    prefix_format: MpdFormatter,
    suffix_format: MpdFormatter,
    icons: StatusIconsSet,
    default_placeholder: String,
}

impl MpdSource {
    pub fn new(
        addr: SocketAddr,
        fmt: MpdFormatter,
        prefix: MpdFormatter,
        suffix: MpdFormatter,
        icons: StatusIconsSet,
        default_placeholder: String,
    ) -> Self {
        Self {
            client: Client::connect(addr).expect("MPD connection error"),
            current_song: None,
            current_status: None,
            running_format: fmt,
            prefix_format: prefix,
            suffix_format: suffix,
            icons,
            default_placeholder,
        }
    }
    pub fn get(
        &mut self,
        content: &mut String,
        prefix: &mut String,
        suffix: &mut String,
    ) -> Result<bool, fmt::Error> {
        let song = self.client.currentsong().expect("MPD server error");
        let status = self.client.status().expect("MPD server error");
        // TODO: May go iterate the formatter to inspect actual changes
        if let Some(s) = song.as_ref() {
            if !self.prefix_format.is_constant() {
                prefix.clear();
                self.prefix_format.format(
                    &self.icons,
                    s,
                    &status,
                    &self.default_placeholder,
                    prefix,
                )?;
            }
            if !self.suffix_format.is_constant() {
                suffix.clear();
                self.suffix_format.format(
                    &self.icons,
                    s,
                    &status,
                    &self.default_placeholder,
                    suffix,
                )?;
            }
        }
        if song == self.current_song {
            return Ok(false);
        }
        content.clear();
        if let Some(s) = song.as_ref() {
            self.running_format.format(
                &self.icons,
                s,
                &status,
                &self.default_placeholder,
                content,
            )?;
        } else {
            write!(content, "{}", self.default_placeholder)?;
        }
        self.current_song = song;
        self.current_status = Some(status);
        Ok(true)
    }
    pub fn running_format(&self) -> &MpdFormatter {
        &self.running_format
    }
    pub fn prefix_format(&self) -> &MpdFormatter {
        &self.prefix_format
    }
    pub fn suffix_format(&self) -> &MpdFormatter {
        &self.suffix_format
    }
    pub fn icons(&self) -> &StatusIconsSet {
        &self.icons
    }
    pub fn current_song(&self) -> Option<&Song> {
        self.current_song.as_ref()
    }
    pub fn current_status(&self) -> Option<&Status> {
        self.current_status.as_ref()
    }
}

impl MpdFormatter {
    pub fn only_string(str: String) -> Self {
        Self(vec![Placeholder::String(str)])
    }
    pub fn is_constant(&self) -> bool {
        self.iter().all(|ph| matches!(ph, Placeholder::String(_)))
    }
    pub fn format_with_source(&self, source: &MpdSource, f: &mut String) -> std::fmt::Result {
        if let (Some(song), Some(status)) = (&source.current_song, &source.current_status) {
            self.format(&source.icons, song, status, &source.default_placeholder, f)
        } else {
            Ok(())
        }
    }
    pub fn format(
        &self,
        icons: &StatusIconsSet,
        song: &Song,
        status: &Status,
        default: &str,
        f: &mut String,
    ) -> std::fmt::Result {
        let mut tags: HashMap<&str, &str> = song
            .tags
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        for ph in self.iter() {
            match ph {
                Placeholder::String(s) => write!(f, "{}", s),
                Placeholder::Artist => write!(f, "{}", song.artist.as_deref().unwrap_or(default)),
                Placeholder::AlbumArtist => {
                    write!(f, "{}", tags.remove("albumartist").unwrap_or(default))
                }
                Placeholder::Album => write!(f, "{}", tags.remove("album").unwrap_or(default)),
                Placeholder::Title => write!(f, "{}", song.title.as_deref().unwrap_or(default)),
                Placeholder::Filename => write!(f, "{}", &song.file),
                Placeholder::Date => write!(f, "{}", tags.remove("date").unwrap_or(default)),
                Placeholder::Volume => write!(f, "{}", status.volume),
                Placeholder::ElapsedTime => match status.elapsed {
                    Some(e) => write!(f, "{:02}:{:02}", e.as_secs() / 60, e.as_secs() % 60),
                    None => write!(f, "{}", default),
                },
                Placeholder::TotalTime => match status.duration {
                    Some(d) => write!(f, "{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60),
                    None => write!(f, "{}", default),
                },
                Placeholder::SongPosition => {
                    let pos = status.song.map(|p| p.pos.to_string());
                    write!(f, "{}", pos.as_deref().unwrap_or(default))
                }
                Placeholder::QueueLength => write!(f, "{}", status.queue_len.to_string()),
                Placeholder::StateIcon => write!(f, "{}", icons.state.get_icon(status.state)),
                Placeholder::ConsumeIcon => icons.consume.write(status.consume, f),
                Placeholder::RandomIcon => icons.random.write(status.random, f),
                Placeholder::RepeatIcon => icons.repeat.write(status.repeat, f),
                Placeholder::SingleIcon => icons.single.write(status.single, f),
            }?;
        }
        Ok(())
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
                        Placeholder::ConsumeIcon => "{consumeIcon}",
                        Placeholder::Date => "{date}",
                        Placeholder::ElapsedTime => "{elapsedTime}",
                        Placeholder::Filename => "{filename}",
                        Placeholder::QueueLength => "{queueLength}",
                        Placeholder::RandomIcon => "{randomIcon}",
                        Placeholder::RepeatIcon => "{repeatIcon}",
                        Placeholder::SingleIcon => "{singleIcon}",
                        Placeholder::SongPosition => "{songPosition}",
                        Placeholder::StateIcon => "{stateIcon}",
                        Placeholder::Title => "{title}",
                        Placeholder::TotalTime => "{totalTime}",
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
            placeholders.push(match &parse_slice[..right_par] {
                "album" => Placeholder::Album,
                "albumArtist" => Placeholder::AlbumArtist,
                "artist" => Placeholder::Artist,
                "consumeIcon" => Placeholder::ConsumeIcon,
                "date" => Placeholder::Date,
                "elapsedTime" => Placeholder::ElapsedTime,
                "filename" => Placeholder::Filename,
                "queueLength" => Placeholder::QueueLength,
                "randomIcon" => Placeholder::RandomIcon,
                "repeatIcon" => Placeholder::RepeatIcon,
                "singleIcon" => Placeholder::SingleIcon,
                "songPosition" => Placeholder::SongPosition,
                "stateIcon" => Placeholder::StateIcon,
                "title" => Placeholder::Title,
                "totalTime" => Placeholder::TotalTime,
                "volume" => Placeholder::Volume,
                _ => {
                    return Err(MpdFormatParseError::UnknownPlaceholder(
                        parse_slice[..right_par].to_owned(),
                    ))
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
    macro_rules! ph {
        ($p:ident) => {
            Placeholder::$p
        };
        ($str:literal) => {
            Placeholder::String($str.to_owned())
        };
    }
    #[test]
    fn format_parse_test() {
        macro_rules! assert_ok {
            ($str:literal => [$($item:tt),*]) => {
                assert_eq!($str.parse::<MpdFormatter>().unwrap().0, vec![$(ph!($item)),*])
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
