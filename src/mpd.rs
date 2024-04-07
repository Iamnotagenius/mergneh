use std::{error::Error, fmt::Display, net::SocketAddr, str::FromStr};

use mpd::{Client, Song};

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

#[derive(Clone)]
pub struct PlayerStatusIcons {
    play: char,
    pause: char,
    stop: char,
}

#[derive(Clone)]
pub struct StatusIcons {
    enabled: char,
    disabled: char,
}

#[derive(Debug, Clone, Default)]
pub struct MpdFormat(Vec<Placeholder>);

#[derive(Debug)]
pub enum MpdFormatParseError {
    UnknownPlaceholder(String),
    UnmatchedParenthesis,
}

#[derive(Debug, PartialEq, Clone)]
enum Placeholder {
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

pub struct MpdSource {
    client: Client,
    current_song: Option<Song>,
    running_format: MpdFormat,
    prefix_format: Option<MpdFormat>,
    suffix_format: Option<MpdFormat>,
}

impl MpdSource {
    pub fn new(addr: SocketAddr, fmt: MpdFormat, prefix: Option<MpdFormat>, suffix: Option<MpdFormat>) -> Self {
        Self {
            client: Client::connect(addr).expect("MPD connection error"),
            current_song: None,
            running_format: fmt,
            prefix_format: prefix,
            suffix_format: suffix,
        }
    }
}

impl Display for MpdFormatParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MpdFormatParseError::UnknownPlaceholder(placeholder) => write!(f, "Unknown placeholder '{placeholder}'"),
            MpdFormatParseError::UnmatchedParenthesis => write!(f, "Unmatched '{{' or '}}"),
        }
    }
}
impl Error for MpdFormatParseError {}

impl Display for MpdFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ph in self.0.iter() {
            if let Placeholder::String(s) = ph {
                for part in s.split_inclusive(['{', '}']) {
                    write!(f, "{}", part)?;
                    match part.chars().last().expect("Part must not be empty") {
                        c if matches!(c, '{' | '}') => write!(f, "{}", c)?,
                        _ => continue
                    };
                }
            } else {
            write!(f, "{}", match ph {
                Placeholder::Album        =>        "{album}",
                Placeholder::AlbumArtist  =>  "{albumArtist}",
                Placeholder::Artist       =>       "{artist}",
                Placeholder::ConsumeIcon  =>  "{consumeIcon}",
                Placeholder::Date         =>         "{date}",
                Placeholder::ElapsedTime  =>  "{elapsedTime}",
                Placeholder::Filename     =>     "{filename}",
                Placeholder::QueueLength  =>  "{queueLength}",
                Placeholder::RandomIcon   =>   "{randomIcon}",
                Placeholder::RepeatIcon   =>   "{repeatIcon}",
                Placeholder::SingleIcon   =>   "{singleIcon}",
                Placeholder::SongPosition => "{songPosition}",
                Placeholder::StateIcon    =>    "{stateIcon}",
                Placeholder::Title        =>        "{title}",
                Placeholder::TotalTime    =>    "{totalTime}",
                Placeholder::Volume       =>       "{volume}",
                Placeholder::String(_)    =>   unreachable!(),
            })?;
            }
        };
        Ok(())
    }
}

impl FromStr for MpdFormat {
    type Err = MpdFormatParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut placeholders = Vec::new();
        let mut raw = String::new();
        let mut parse_slice = dbg!(s);
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
                None => return Err(MpdFormatParseError::UnmatchedParenthesis)
            };
            if let Some('{') = parse_slice[right_par..].chars().next() {
                return Err(MpdFormatParseError::UnmatchedParenthesis);
            }
            placeholders.push(match &parse_slice[..right_par] {
                "album"         =>        Placeholder::Album,
                "albumArtist"   =>  Placeholder::AlbumArtist,
                "artist"        =>       Placeholder::Artist,
                "consumeIcon"   =>  Placeholder::ConsumeIcon,
                "date"          =>         Placeholder::Date,
                "elapsedTime"   =>  Placeholder::ElapsedTime,
                "filename"      =>     Placeholder::Filename,
                "queueLength"   =>  Placeholder::QueueLength,
                "randomIcon"    =>   Placeholder::RandomIcon,
                "repeatIcon"    =>   Placeholder::RepeatIcon,
                "singleIcon"    =>   Placeholder::SingleIcon,
                "songPosition"  => Placeholder::SongPosition,
                "stateIcon"     =>    Placeholder::StateIcon,
                "title"         =>        Placeholder::Title,
                "totalTime"     =>    Placeholder::TotalTime,
                "volume"        =>       Placeholder::Volume,
                _ => return Err(MpdFormatParseError::UnknownPlaceholder(parse_slice[..right_par].to_owned()))
            });
            parse_slice = &parse_slice[right_par + 1..];
        }
        dbg!(&raw);
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

pub fn parse_player_icons(icons: &str) -> Result<PlayerStatusIcons, IconSetParseError<3>> {
    let mut iter = icons.chars();
    let result = Ok(next_or_err!(iter => PlayerStatusIcons: play, pause, stop));
    if iter.next().is_some() {
        return Err(IconSetParseError::TooManyChars);
    }
    result
}

pub fn parse_status_icons(icons: &str) -> Result<StatusIcons, IconSetParseError<2>> {
    let mut iter = icons.chars();
    let result = Ok(next_or_err!(iter => StatusIcons: enabled, disabled));
    if iter.next().is_some() {
        return Err(IconSetParseError::TooManyChars);
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::mpd::{MpdFormat, Placeholder, MpdFormatParseError};
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
                assert_eq!($str.parse::<MpdFormat>().unwrap().0, vec![$(ph!($item)),*])
            };
        }
        macro_rules! assert_err {
            ($str:literal => $err:ident$(($s:literal))?) => {
                assert!(matches!($str.parse::<MpdFormat>().unwrap_err(), MpdFormatParseError::$err$((s) if s.as_str() == $s)?));
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
                assert_eq!(MpdFormat(vec![$(ph!($item)),*]).to_string(), $str)
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
                assert_eq!($str.parse::<MpdFormat>().unwrap().to_string(), $str)
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
