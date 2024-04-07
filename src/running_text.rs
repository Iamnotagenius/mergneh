use std::{
    io::{self, Write},
    time::Duration,
};

use ticker::Ticker;

use crate::{utils::replace_newline, TextSource};

pub struct RunningText {
    source: TextSource,
    content: String,
    prefix: String,
    suffix: String,
    window_size: usize,
    repeat: bool,
    text: String,
    full_content_char_len: usize,
    content_char_len: usize,
    i: usize,
    byte_offset: usize,
}

impl RunningText {
    pub fn new(
        mut source: TextSource,
        window_size: usize,
        mut separator: String,
        newline: String,
        prefix: String,
        suffix: String,
        repeat: bool,
    ) -> Result<Self, io::Error> {
        let mut content: String = source.get_content();
        replace_newline(&mut content, &newline);
        replace_newline(&mut separator, &newline);
        let content_len = content.len();
        let count = content[..content_len].chars().count();
        content += &separator;
        Ok(RunningText {
            source,
            text: if !repeat && window_size >= count {
                let mut full = prefix.clone();
                full.push_str(&content[..content_len]);
                full.push_str(&suffix);
                full
            } else {
                String::new()
            },
            full_content_char_len: count + content[content_len..].chars().count(),
            content,
            prefix,
            suffix,
            window_size,
            repeat,
            content_char_len: count,
            i: 0,
            byte_offset: 0,
        })
    }
    pub fn get_raw_content(&self) -> &str {
        &self.content
    }
    pub fn run_on_terminal(self, duration: Duration) -> Result<(), io::Error> {
        let tick = Ticker::new(self, duration);
        for text in tick {
            print!("\r{}", text);
            io::stdout().flush()?;
        }
        Ok(())
    }
    pub fn print_once(&mut self, mut i: usize, prev_content: &str) -> usize {
        if prev_content != self.content {
            i = 0;
        }
        self.i = i;
        self.byte_offset = self.content.char_indices().nth(i % self.full_content_char_len).unwrap().0;
        println!("{}", self.next().unwrap());
        self.i
    }
    fn does_content_fit(&self) -> bool {
        !self.repeat && self.window_size >= self.content_char_len
    }
}

impl Iterator for RunningText {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.does_content_fit() {
            return Some(self.text.to_owned());
        }
        self.text.clear();
        self.text.push_str(&self.prefix);
        self.text.extend(
            self.content[self.byte_offset..]
                .chars()
                .take(self.window_size),
        );

        let mut remainder = self
            .window_size
            .saturating_sub(self.full_content_char_len - self.i);
        while remainder >= self.full_content_char_len {
            self.text.push_str(&self.content);
            remainder -= self.full_content_char_len;
        }
        self.text.extend(self.content.chars().take(remainder));
        self.i += 1;
        self.i %= self.full_content_char_len;
        self.byte_offset += &self.content[self.byte_offset..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or_default();
        self.byte_offset %= self.content.len();
        self.text.push_str(&self.suffix);
        Some(self.text.clone())
    }
}
