use std::{
    io::{self, Write},
    time::Duration,
};

use ticker::Ticker;

use crate::TextSource;

pub struct RunningText {
    source: String,
    duration: Duration,
    window_size: usize,
}

pub struct RunningTextIter<'a> {
    src: &'a RunningText,
    text: String,
    char_count: usize,
    i: usize,
    byte_offset: usize,
}

impl RunningText {
    pub fn new(
        source: TextSource,
        duration: Duration,
        window_size: usize,
    ) -> Result<Self, io::Error> {
        let str = source.try_into()?;
        Ok(RunningText {
            source: str,
            duration,
            window_size,
        })
    }
    pub fn run_on_console(self) -> Result<(), io::Error> {
        let tick = Ticker::new(self.into_iter(), self.duration);
        println!("Source: {}", self.source);
        for w in tick {
            print!("\r{}", w);
            io::stdout().flush()?;
        }
        return Ok(());
    }
}

impl<'a> IntoIterator for &'a RunningText {
    type Item = String;

    type IntoIter = RunningTextIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RunningTextIter {
            src: self,
            text: self.source.chars().take(self.window_size).collect(),
            i: 0usize,
            byte_offset: 0usize,
            char_count: self.source.chars().count(),
        }
    }
}

impl<'a> Iterator for RunningTextIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.text = String::new();
        // 0123
        // 01230123012301
        // 12301230123012
        self.text.extend(
            self.src.source[self.byte_offset..]
                .chars()
                .take(self.src.window_size)
                // TODO: Separator
                .map(replace_newline),
        );

        let mut remainder = self
            .src
            .window_size
            .saturating_sub(self.char_count - self.i);
        while remainder >= self.char_count {
            self.text
                .extend(self.src.source.chars().map(replace_newline)); // TODO: some special case, should be handled more gracefully

            remainder -= self.char_count;
        }
        self.text
            .extend(self.src.source.chars().take(remainder).map(replace_newline));
        self.i += 1;
        self.i %= self.char_count;
        self.byte_offset = (self.byte_offset + 1..self.src.source.len())
            .skip_while(|&i| !self.src.source.is_char_boundary(i))
            .take(1)
            .next()
            .unwrap_or(0);
        Some(self.text.clone())
    }
}
fn replace_newline(c: char) -> char {
    match c {
        '\n' => '',
        _ => c,
    }
}
