use std::{fmt::Write};

use crate::{
    utils::replace_newline,
    TextSource,
};

#[derive(Debug)]
pub struct RunningText {
    source: TextSource,
    content: String,
    newline: String,
    separator: String,
    replacements: Vec<(String, String)>,
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
        replacements: Vec<(String, String)>,
        repeat: bool,
        reset_on_change: bool,
    ) -> anyhow::Result<Self> {
        let mut content = source.get_initial_content()?;
        replace_newline(&mut content, &newline);
        replace_newline(&mut separator, &newline);
        let content_len = content.len();
        let count = content.chars().count();
        content += &separator;
        let mut new = RunningText {
            source,
            text: String::new(),
            full_content_char_len: count + content[content_len..].chars().count(),
            content,
            newline,
            separator,
            replacements,
            window_size,
            repeat,
            reset_on_change,
            content_char_len: count,
            i: 0,
            byte_offset: 0,
        };
        if new.does_content_fit() {
            new.text.write_str(&new.content[..content_len])?;
            new.apply_replacements();
        }
        Ok(new)
    }
    fn does_content_fit(&self) -> bool {
        !self.repeat && self.window_size >= self.content_char_len
    }
    fn apply_replacements(&mut self) {
        for (src, dest) in self.replacements.iter() {
            let ranges = self
                .text
                .match_indices(src)
                .enumerate()
                .map(|(i, (j, m))| {
                    let diff = (dest.len() as isize - src.len() as isize) * i as isize;
                    j.saturating_add_signed(diff)..(j + m.len()).saturating_add_signed(diff)
                })
                .collect::<Vec<_>>();
            for range in ranges {
                self.text.replace_range(range, dest);
            }
        }
    }
    fn get_new_content(&mut self) -> anyhow::Result<bool> {
        let changed = self.source.get_content(
            &mut self.content,
        )?;
        if !changed {
            return Ok(false);
        }
        replace_newline(&mut self.content, &self.newline);
        self.content_char_len = self.content.chars().count();
        self.content += &self.separator;
        self.full_content_char_len = self.content_char_len + self.separator.chars().count();
        if self.reset_on_change {
            self.i = 0;
            self.byte_offset = 0;
        } else {
            self.i %= self.full_content_char_len;
            self.byte_offset = self.content.char_indices().nth(self.i).unwrap().0;
        }
        Ok(true)
    }
}

impl Iterator for RunningText {
    type Item = anyhow::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let changed = match self.get_new_content() {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };
        if self.content.is_empty() {
            return None;
        }
        if self.does_content_fit() {
            if changed {
                self.text.clear();
                if let Err(e) = self.text.write_str(&self.content[..self.content.len() - self.separator.len()]) {
                    return Some(Err(e.into()));
                };
                self.apply_replacements();
            }
            return Some(Ok(self.text.to_owned()));
        }
        self.text.clear();
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
        self.apply_replacements();
        Some(Ok(self.text.clone()))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Ok, Result};

    use crate::text_source::TextSource;

    use super::RunningText;

    macro_rules! assert_text {
        ($var:ident, $($iter:literal),+) => {
            $(assert_eq!($var.next().unwrap()?, $iter));+
        }
    }

    #[test]
    fn one_full_cycle() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content(
                "I am a running text".to_owned(),
            ),
            12,
            "|".to_owned(),
            "".to_owned(),
            vec![],
            false,
            false,
        )?;
        assert_text!(
            text,
            "I am a runni",
            " am a runnin",
            "am a running",
            "m a running ",
            " a running t",
            "a running te",
            " running tex",
            "running text",
            "unning text|",
            "nning text|I",
            "ning text|I ",
            "ing text|I a",
            "ng text|I am",
            "g text|I am ",
            " text|I am a",
            "text|I am a ",
            "ext|I am a r",
            "xt|I am a ru",
            "t|I am a run",
            "|I am a runn",
            "I am a runni"
        );
        Ok(())
    }

    #[test]
    fn with_repeat() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content(
                "I am a running text".to_owned(),
            ),
            25,
            "|".to_owned(),
            "".to_owned(),
            vec![],
            true,
            false,
        )?;
        assert_text!(
            text,
            "I am a running text|I am ",
            " am a running text|I am a",
            "am a running text|I am a ",
            "m a running text|I am a r",
            " a running text|I am a ru",
            "a running text|I am a run",
            " running text|I am a runn",
            "running text|I am a runni",
            "unning text|I am a runnin",
            "nning text|I am a running",
            "ning text|I am a running ",
            "ing text|I am a running t",
            "ng text|I am a running te",
            "g text|I am a running tex",
            " text|I am a running text",
            "text|I am a running text|",
            "ext|I am a running text|I",
            "xt|I am a running text|I ",
            "t|I am a running text|I a",
            "|I am a running text|I am",
            "I am a running text|I am "
        );
        Ok(())
    }

    #[test]
    fn special_chars() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content("?#@!$%^^&*()".to_owned()),
            12,
            "".to_owned(),
            "".to_owned(),
            vec![],
            true,
            false,
        )?;
        assert_text!(
            text,
            "$ ?#@!$%^^&*() &<",
            "$ #@!$%^^&*()? &<",
            "$ @!$%^^&*()?# &<",
            "$ !$%^^&*()?#@ &<",
            "$ $%^^&*()?#@! &<",
            "$ %^^&*()?#@!$ &<",
            "$ ^^&*()?#@!$% &<",
            "$ ^&*()?#@!$%^ &<",
            "$ &*()?#@!$%^^ &<",
            "$ *()?#@!$%^^& &<",
            "$ ()?#@!$%^^&* &<",
            "$ )?#@!$%^^&*( &<",
            "$ ?#@!$%^^&*() &<",
            "$ #@!$%^^&*()? &<",
            "$ @!$%^^&*()?# &<",
            "$ !$%^^&*()?#@ &<"
        );
        Ok(())
    }

    #[test]
    fn replacement() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content("?#@!$%^^&*()".to_owned()),
            12,
            "".to_owned(),
            "".to_owned(),
            vec![
                ("&".to_owned(), "&amp".to_owned()),
                ("()".to_owned(), "b".to_owned()),
            ],
            true,
            false,
        )?;
        assert_text!(
            text,
            "?#@!$%^^&amp*b",
            "#@!$%^^&amp*b?",
            "@!$%^^&amp*b?#",
            "!$%^^&amp*b?#@",
            "$%^^&amp*b?#@!",
            "%^^&amp*b?#@!$",
            "^^&amp*b?#@!$%",
            "^&amp*b?#@!$%^",
            "&amp*b?#@!$%^^",
            "*b?#@!$%^^&amp",
            "b?#@!$%^^&amp*",
            ")?#@!$%^^&amp*(",
            "?#@!$%^^&amp*b",
            "#@!$%^^&amp*b?",
            "@!$%^^&amp*b?#",
            "!$%^^&amp*b?#@"
        );
        Ok(())
    }

    #[test]
    fn without_repeat() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content("a & b".to_owned()),
            5,
            "".to_owned(),
            "".to_owned(),
            vec![],
            false,
            false,
        )?;
        assert!(text.does_content_fit());
        assert_text!(text, "a & b", "a & b", "a & b", "a & b");
        Ok(())
    }

    #[test]
    fn replacement_without_repeat() -> Result<()> {
        let mut text = RunningText::new(
            TextSource::content("a & b".to_owned()),
            5,
            "|".to_owned(),
            "".to_owned(),
            vec![("&".to_owned(), "&amp;".to_owned())],
            false,
            false,
        )?;
        assert!(text.does_content_fit());
        assert_text!(text, "a &amp; b", "a &amp; b", "a &amp; b", "a &amp; b");
        Ok(())
    }
}
