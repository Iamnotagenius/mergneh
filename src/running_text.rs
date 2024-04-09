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
    reset_on_change: bool,
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
        reset_on_change: bool,
    ) -> anyhow::Result<Self> {
        let Content {
            running: mut content,
            prefix,
            suffix,
        } = source.get_initial_content()?;
        replace_newline(&mut content, &newline);
        replace_newline(&mut separator, &newline);
        let content_len = content.len();
        let count = content[..content_len].chars().count();
        content += &separator;
        Ok(RunningText {
            source,
            text: if !repeat && window_size >= count {
                format!("{prefix}{}{suffix}", &content[..content_len])
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
            reset_on_change,
            content_char_len: count,
            i: 0,
            byte_offset: 0,
        })
    }
    pub fn get_raw_content(&self) -> &str {
        &self.content
    }
    pub fn run_on_terminal(self, duration: Duration) -> anyhow::Result<()> {
        let tick = Ticker::new(self, duration);
        for text in tick {
            print!("\r{}", text?);
            io::stdout().flush()?;
        }
        Ok(())
    }
    pub fn print_once(&mut self, mut i: usize, prev_content: &str) -> anyhow::Result<usize> {
        if prev_content != self.content {
            i = 0;
        }
        self.i = i;
        self.byte_offset = self
            .content
            .char_indices()
            .nth(self.i % self.full_content_char_len)
            .unwrap()
            .0;
        println!("{}", self.next().unwrap()?);
        Ok(self.i)
    }
    #[cfg(feature = "waybar")]
    pub fn with_tooltip(self, tooltip: Tooltip) -> RunningTextWithTooltip {
        RunningTextWithTooltip::new(self, tooltip)
    }
    #[cfg(feature = "waybar")]
    pub fn run_in_waybar(self, duration: Duration, tooltip: Option<Tooltip>) -> anyhow::Result<()> {
        match tooltip {
            Some(Tooltip::Simple(s)) => {
                let tick = Ticker::new(self, duration);
                for text in tick {
                    println!("{{\"text\":\"{}\",\"tooltip\":\"{}\"}}", text?, s);
                }
            }
            Some(t) => {
                let tick = Ticker::new(self.with_tooltip(t), duration);
                for (text, tt) in tick {
                    println!("{{\"text\":\"{}\",\"tooltip\":\"{}\"}}", text?, tt);
                }
            }
            None => {
                let tick = Ticker::new(self, duration);
                for text in tick {
                    println!("{{\"text\":\"{}\"}}", text?);
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
    fn get_new_content(&mut self) -> anyhow::Result<ContentChange> {
        let changes =
            self.source
                .get_content(&mut self.content, &mut self.prefix, &mut self.suffix)?;
        if !changes.contains(ContentChange::Running) {
            return Ok(changes);
        }
        // TODO: not always reset pos on content change
        replace_newline(&mut self.content, &self.newline);
        let content_len = self.content.len();
        self.content_char_len = self.content.chars().count();
        self.content.push_str(&self.separator);
        self.full_content_char_len = self.content_char_len + self.separator.chars().count();
        if self.reset_on_change {
            (self.i, self.byte_offset) = (0, 0);
        } else {
            self.i %= self.full_content_char_len;
            self.byte_offset = self.content.char_indices().nth(self.i).unwrap().0;
        }
        self.text = if self.does_content_fit() {
            format!(
                "{}{}{}",
                self.prefix,
                &self.content[..content_len],
                self.suffix
            )
        } else {
            String::new()
        };
        Ok(changes)
    }
}

impl Iterator for RunningText {
    type Item = anyhow::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let changes = match self.get_new_content() {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        if self.content.is_empty() {
            return None;
        }
        if self.does_content_fit() {
            if !changes.is_empty() {
                self.text = self.prefix.clone()
                    + (&self.content[..self.content.len() - self.separator.len()])
                    + &self.suffix;
            }
            return Some(Ok(self.text.to_owned()));
        }
        self.text.clone_from(&self.prefix);
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
        self.i = (self.i + 1) % self.full_content_char_len;
        self.byte_offset += &self.content[self.byte_offset..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or_default();
        self.byte_offset %= self.content.len();
        self.text.push_str(&self.suffix);
        Some(Ok(self.text.clone()))
    }
}
