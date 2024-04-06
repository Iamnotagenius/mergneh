use std::{
    io::{self, Write},
    time::Duration,
};

use ticker::Ticker;

use crate::{utils::replace_newline, TextSource};

pub struct RunningText {
    content: String,
    prefix: String,
    suffix: String,
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
        window_size: usize,
        separator: String,
        newline: String,
        prefix: String,
        suffix: String,
    ) -> Result<Self, io::Error> {
        let mut content = source.try_into()?;
        content += separator.as_str();
        replace_newline(&mut content, newline.as_str());
        Ok(RunningText {
            content,
            prefix,
            suffix,
            window_size,
        })
    }
    pub fn run_on_terminal(self, duration: Duration) -> Result<(), io::Error> {
        let tick = Ticker::new(self.into_iter(), duration);
        for text in tick {
            print!("\r{}{}{}", self.prefix, text, self.suffix);
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
            text: self.content.chars().take(self.window_size).collect(),
            i: 0usize,
            byte_offset: 0usize,
            char_count: self.content.chars().count(),
        }
    }
}

impl<'a> Iterator for RunningTextIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.text = String::new();
        self.text.extend(
            self.src.content[self.byte_offset..]
                .chars()
                .take(self.src.window_size),
        );

        let mut remainder = self
            .src
            .window_size
            .saturating_sub(self.char_count - self.i);
        while remainder >= self.char_count {
            self.text.extend(self.src.content.chars()); // TODO: some special case, should be handled more gracefully

            remainder -= self.char_count;
        }
        self.text.extend(self.src.content.chars().take(remainder));
        self.i += 1;
        self.i %= self.char_count;
        self.byte_offset = (self.byte_offset + 1..self.src.content.len())
            .skip_while(|&i| !self.src.content.is_char_boundary(i))
            .take(1)
            .next()
            .unwrap_or(0);
        Some(self.text.clone())
    }
}
