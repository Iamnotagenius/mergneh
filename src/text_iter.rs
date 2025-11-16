use crate::{running_text::RunningText, text_source::TextSource};

pub struct TextIter {
    source: Box<dyn TextSource>,
    w: usize,
    repeat: bool,
    separator: String,
    replacements: Vec<(String, String)>,
    right: bool,
}

impl TextIter {
    pub fn new(
        source: Box<dyn TextSource>,
        w: usize,
        repeat: bool,
        separator: String,
        replacements: Vec<(String, String)>,
        right: bool,
    ) -> Self {
        Self {
            source,
            w,
            repeat,
            separator,
            replacements,
            right,
        }
    }

    pub fn source(&mut self) -> &mut Box<dyn TextSource> {
        &mut self.source
    }

    pub fn right(&self) -> bool {
        self.right
    }

    pub fn new_text(&self, mut content: String) -> RunningText {
        if self.repeat || content.chars().count() > self.w {
            content.push_str(&self.separator);
        }

        RunningText::new(
            content,
            self.w,
            self.repeat,
            &self.replacements,
        )
    }
}
