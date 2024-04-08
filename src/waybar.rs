use crate::text_source::TextSource;
#[cfg(feature = "mpd")]
use crate::mpd::MpdFormatter;

use super::RunningText;

#[derive(Debug)]
pub enum Tooltip {
    Simple(String),
    #[cfg(feature = "mpd")]
    Mpd(MpdFormatter)
}
pub struct RunningTextWithTooltip {
    text: RunningText,
    tooltip: Tooltip,
    buffer: String,
}

impl RunningTextWithTooltip {
    pub fn new(text: RunningText, tooltip: Tooltip) -> RunningTextWithTooltip {
        RunningTextWithTooltip { text, tooltip, buffer: String::new() }
    }
}

impl Iterator for RunningTextWithTooltip {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        let iteration = self.text.next().unwrap();
        let src = self.text.get_source();
        let tooltip = match (&self.tooltip, src) {
            (Tooltip::Simple(s), _) => s,
            #[cfg(feature = "mpd")]
            (Tooltip::Mpd(f), TextSource::Mpd(s)) => {
                self.buffer.clear();
                f.format_with_source(s, &mut self.buffer).expect("MPD format error");
                &self.buffer
            }
            #[cfg(feature = "mpd")]
            (Tooltip::Mpd(_), TextSource::String(_)) => panic!("I refuse."),
        };
        Some((iteration, tooltip.to_owned()))
    }
}
