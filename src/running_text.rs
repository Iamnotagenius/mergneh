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
    repeat: bool,
    separator_boundary: usize,
}

pub struct RunningTextIter<'a> {
    src: &'a RunningText,
    text: String,
    content_and_sep_count: usize,
    content_count: usize,
    i: usize,
    byte_offset: usize,
}

impl RunningText {
    pub fn new(
        source: TextSource,
        window_size: usize,
        mut separator: String,
        newline: String,
        prefix: String,
        suffix: String,
        repeat: bool,
    ) -> Result<Self, io::Error> {
        let mut content: String = source.try_into()?;
        replace_newline(&mut content, &newline);
        replace_newline(&mut separator, &newline);
        let content_len = content.len();
        content += &separator;
        Ok(RunningText {
            separator_boundary: content_len,
            content,
            prefix,
            suffix,
            window_size,
            repeat,
        })
    }
    pub fn get_raw_content(&self) -> &str {
        &self.content
    }
    pub fn run_on_terminal(self, duration: Duration) -> Result<(), io::Error> {
        let tick = Ticker::new(self.into_iter(), duration);
        for text in tick {
            print!("\r{}{}{}", self.prefix, text, self.suffix);
            io::stdout().flush()?;
        }
        return Ok(());
    }
    pub fn print_once(&self, mut i: usize, prev_content: &str) -> usize {
        if prev_content != self.content {
            i = 0;
        }
        let count = self.content[..self.separator_boundary].chars().count();
        let mut iter = RunningTextIter {
            src: self,
            text: if !self.repeat && self.window_size >= count {
                self.content[..self.separator_boundary].to_owned()
            } else {
                String::new()
            },
            content_and_sep_count: count + self.content[self.separator_boundary..].chars().count(),
            content_count: count,
            i,
            byte_offset: self.content.char_indices().nth(i % count).unwrap().0,
        };
        println!("{}{}{}", self.prefix, iter.next().unwrap(), self.suffix);
        return iter.i;
    }
}

impl<'a> IntoIterator for &'a RunningText {
    type Item = String;

    type IntoIter = RunningTextIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let count = self.content[..self.separator_boundary].chars().count();
        RunningTextIter {
            src: self,
            text: if !self.repeat && self.window_size >= count {
                self.content[..self.separator_boundary].to_owned()
            } else {
                String::new()
            },
            i: 0usize,
            byte_offset: 0usize,
            content_and_sep_count: count + self.content[self.separator_boundary..].chars().count(),
            content_count: count,
        }
    }
}

impl<'a> Iterator for RunningTextIter<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.src.repeat && self.src.window_size >= self.content_count {
            return Some(self.text.to_owned());
        }
        self.text.clear();
        self.text.extend(
            self.src.content[self.byte_offset..]
                .chars()
                .take(self.src.window_size),
        );

        let mut remainder = self
            .src
            .window_size
            .saturating_sub(self.content_and_sep_count - self.i);
        while remainder >= self.content_and_sep_count {
            self.text.extend(self.src.content.chars()); // TODO: some special case, should be handled more gracefully

            remainder -= self.content_and_sep_count;
        }
        self.text.extend(self.src.content.chars().take(remainder));
        self.i += 1;
        self.i %= self.content_and_sep_count;
        self.byte_offset += &self.src.content[self.byte_offset..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or_default();
        self.byte_offset %= self.src.content.len();
        Some(self.text.clone())
    }
}
