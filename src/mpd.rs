use std::{error::Error, fmt::Display};

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
