use std::{
    io::{self, Write},
    time::Duration,
};

use ticker::Ticker;

#[cfg(feature = "waybar")]
use crate::waybar::{RunningTextWithTooltip, Tooltip};
use crate::{
    text_source::{Content, ContentChange},
    utils::replace_newline,
    TextSource,
};

pub struct RunningText {
    source: TextSource,
    content: String,
    newline: String,
    separator: String,
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
        repeat: bool,
    ) -> Result<Self, io::Error> {
        let Content {
            running: mut content,
            prefix,
            suffix,
        } = source.get_initial_content();
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
            newline,
            separator,
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
    pub fn run_on_terminal(self, duration: Duration) -> io::Result<()> {
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
        self.byte_offset = self
            .content
            .char_indices()
            .nth(i % self.full_content_char_len)
            .unwrap()
            .0;
        println!("{}", self.next().unwrap());
        self.i
    }
    #[cfg(feature = "waybar")]
    pub fn with_tooltip(self, tooltip: Tooltip) -> RunningTextWithTooltip {
        RunningTextWithTooltip::new(self, tooltip)
    }
    #[cfg(feature = "waybar")]
    pub fn run_in_waybar(self, duration: Duration, tooltip: Option<Tooltip>) -> io::Result<()> {
        match tooltip {
            Some(Tooltip::Simple(s)) => {
                let tick = Ticker::new(self, duration);
                for text in tick {
                    println!("{{\"text\":\"{}\",\"tooltip\":\"{}\"}}", text, s);
                }
            }
            Some(t) => {
                let tick = Ticker::new(self.with_tooltip(t), duration);
                for (text, tt) in tick {
                    println!("{{\"text\":\"{}\",\"tooltip\":\"{}\"}}", text, tt);
                }
            }
            None => {
                let tick = Ticker::new(self, duration);
                for text in tick {
                    println!("{{\"text\":\"{}\"}}", text);
                }
            }
        };
        io::stdout().flush()?;
        Ok(())
    }
    pub fn get_source(&self) -> &TextSource {
        &self.source
    }
    fn does_content_fit(&self) -> bool {
        !self.repeat && self.window_size >= self.content_char_len
    }
    fn get_new_content(&mut self) -> ContentChange {
        let changes =
            self.source
                .get_content(&mut self.content, &mut self.prefix, &mut self.suffix);
        if !changes.contains(ContentChange::Running) {
            return changes;
        }
        // TODO: not always reset pos on content change
        self.i = 0;
        self.byte_offset = 0;
        replace_newline(&mut self.content, &self.newline);
        let content_len = self.content.len();
        self.content_char_len = self.content.chars().count();
        self.content += &self.separator;
        self.full_content_char_len = self.content_char_len + self.separator.chars().count();
        self.text = if self.does_content_fit() {
            let mut full = self.prefix.clone();
            full.push_str(&self.content[..content_len]);
            full.push_str(&self.suffix);
            full
        } else {
            String::new()
        };
        changes
    }
}

impl Iterator for RunningText {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let changes = self.get_new_content();
        if self.does_content_fit() {
            if !changes.is_empty() {
                self.text.clear();
                self.text.push_str(&self.prefix);
                self.text
                    .push_str(&self.content[..self.content.len() - self.separator.len()]);
                self.text.push_str(&self.suffix);
            }
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
