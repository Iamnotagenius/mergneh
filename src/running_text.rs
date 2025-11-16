use std::{collections::BTreeMap, ops::{AddAssign, Range, SubAssign}, slice::SliceIndex, str::CharIndices};

#[derive(Debug)]
pub struct RunningText {
    s: String,
    w: usize,
    repeat: bool,
    left_escape_bounds: BTreeMap<usize, usize>,
    right_escape_bounds: BTreeMap<usize, usize>,
}

impl RunningText {
    pub fn new<S: AsRef<str>>(mut string: String, w: usize, repeat: bool, escapes: &[(S, S)]) -> Self {
        let mut char_count = string.chars().count();
        char_count -= escapes
            .iter()
            .filter_map(|(src, dest)| src.as_ref().len().checked_sub(dest.as_ref().len()).filter(|l| *l > 0))
            .sum::<usize>();
        let repeat = repeat || char_count > w;
        let (q, r) = ((w - 1) / char_count, (w - 1) % char_count);
        let mut left_escape_bounds = BTreeMap::new();
        let mut right_escape_bounds = BTreeMap::new();

        for (src, dest) in escapes.iter().map(|(src, dest)| (src.as_ref(), dest.as_ref())) {
            let matches: Vec<_> = string
                .match_indices(src)
                .enumerate()
                .map(|(i, (m, _))| (m as i64 + i as i64 * (dest.len() as i64 - src.len() as i64)) as usize)
                .collect();

            for &i in &matches {
                string.replace_range(i..i + src.len(), dest);
            }

            if repeat {
                left_escape_bounds.extend(matches.iter().filter_map(|&i| (!dest.is_empty()).then_some((i, dest.len()))));
                right_escape_bounds.extend(matches.iter().filter_map(|&i| (!dest.is_empty()).then_some((i + dest.len(), dest.len()))));
            }
        }
        if !repeat {
            return Self {
                s: string,
                w,
                repeat: false,
                left_escape_bounds,
                right_escape_bounds,
            }
        }
        for _ in 0..q {
            string.extend_from_within(..);
        }
        left_escape_bounds.extend(escapes
            .iter()
            .map(|(_, d)| string
                .match_indices(d.as_ref())
                .filter_map(|(i, _)| (!d.as_ref().is_empty()).then_some((i, d.as_ref().len()))))
            .flatten());
        right_escape_bounds.extend(escapes
            .iter()
            .map(|(_, d)| string
                .match_indices(d.as_ref())
                .filter_map(|(i, _)| (!d.as_ref().is_empty()).then_some((i + d.as_ref().len(), d.as_ref().len()))))
            .flatten());

        let mut off = string.char_indices();
        for _ in 0..r {
            let current_off = off.offset();
            match left_escape_bounds.get(&current_off) {
                Some(&len) => {
                    off.by_ref().skip(len - 1).next();
                    left_escape_bounds.insert(current_off + string.len(), len);
                    right_escape_bounds.insert(
                        current_off +
                        string.len() +
                        len,
                        len
                    );
                },
                None => {
                    off.next();
                },
            };
        }
        string.extend_from_within(..off.offset());
        Self {
            s: string,
            w,
            repeat: true,
            left_escape_bounds,
            right_escape_bounds,
        }
    }

    pub fn iter(&self) -> RunIter<'_> {
        if !self.repeat {
            return RunIter {
                s: &self.s,
                init_left_off: 0,
                init_right_off: self.s.len(),
                left_escape_bounds: &self.left_escape_bounds,
                right_escape_bounds: &self.right_escape_bounds,
                left_off: RunIndex::new(&self.s, 0, &self.left_escape_bounds, &self.right_escape_bounds),
                right_off: RunIndex::new(&self.s, self.s.len(), &self.left_escape_bounds, &self.right_escape_bounds),
            };
        }
        let (left_off, right_off) = {
            let mut left = RunIndex::new(
                &self.s,
                self.s.len(),
                &self.left_escape_bounds,
                &self.right_escape_bounds,
            );
            let left = left
                .advance_back_by(self.w)
                .ok()
                .and_then(|()| left.next_back())
                .unwrap_or_default();
            let mut right = RunIndex::new(
                &self.s,
                0,
                &self.left_escape_bounds,
                &self.right_escape_bounds,
            );
            let right = right
                .advance_by(self.w)
                .ok()
                .and_then(|()| right.next())
                .unwrap_or(self.s.len());
            (left, right)
        };
        RunIter {
            s: &self.s,
            init_left_off: left_off,
            init_right_off: right_off,
            left_escape_bounds: &self.left_escape_bounds,
            right_escape_bounds: &self.right_escape_bounds,
            left_off: RunIndex::new(&self.s, 0, &self.left_escape_bounds, &self.right_escape_bounds),
            right_off: RunIndex::new(&self.s, right_off, &self.left_escape_bounds, &self.right_escape_bounds),
        }
    }

    pub fn iter_at(&self, idx: usize) -> RunIter<'_> {
        if !self.repeat {
            return RunIter {
                s: &self.s,
                init_left_off: 0,
                init_right_off: self.s.len(),
                left_escape_bounds: &self.left_escape_bounds,
                right_escape_bounds: &self.right_escape_bounds,
                left_off: RunIndex::new(&self.s, 0, &self.left_escape_bounds, &self.right_escape_bounds),
                right_off: RunIndex::new(&self.s, self.s.len(), &self.left_escape_bounds, &self.right_escape_bounds),
            };
        }
        let (left_off, right_off) = {
            let mut left = RunIndex::new(
                &self.s,
                self.s.len(),
                &self.left_escape_bounds,
                &self.right_escape_bounds,
            );
            let left = left
                .advance_back_by(self.w)
                .ok()
                .and_then(|()| left.next_back())
                .unwrap_or_default();
            let mut right = RunIndex::new(
                &self.s,
                0,
                &self.left_escape_bounds,
                &self.right_escape_bounds,
            );
            let right = right
                .advance_by(self.w)
                .ok()
                .and_then(|()| right.next())
                .unwrap_or(self.s.len());
            (left, right)
        };
        let mut off = self.left_escape_bounds
            .iter()
            .find_map(|(&i, &len)| (i..i + len).contains(&idx).then_some(i))
            .unwrap_or(self.s.floor_char_boundary(idx));
        let off_right;
        let mut off_right_it = RunIndex::new(&self.s, off, &self.left_escape_bounds, &self.right_escape_bounds);
        if let Some(o) = off_right_it.advance_by(self.w).ok().and_then(|()| off_right_it.next()) {
            off_right = o;
        } else {
            off = 0;
            off_right = right_off;
        }
        RunIter {
            s: &self.s,
            init_left_off: left_off,
            init_right_off: right_off,
            left_escape_bounds: &self.left_escape_bounds,
            right_escape_bounds: &self.right_escape_bounds,
            left_off: RunIndex::new(&self.s, off, &self.left_escape_bounds, &self.right_escape_bounds),
            right_off: RunIndex::new(&self.s, off_right, &self.left_escape_bounds, &self.right_escape_bounds),
        }
    }
}

impl<'a> IntoIterator for &'a RunningText {
    type IntoIter = RunIter<'a>;
    type Item = &'a str;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

struct RunIndex<'a> {
    s: &'a str,
    offset: usize,
    left_escape_bounds: &'a BTreeMap<usize, usize>,
    right_escape_bounds: &'a BTreeMap<usize, usize>,
    end: bool,
}

impl<'a> RunIndex<'a>  {
    pub fn new(
        s: &'a str,
        offset: usize,
        left_escape_bounds: &'a BTreeMap<usize, usize>,
        right_escape_bounds: &'a BTreeMap<usize, usize>,
    ) -> Self {
        Self {
            s,
            offset,
            left_escape_bounds,
            right_escape_bounds,
            end: false,
        }
    }
    pub fn peek(&self) -> usize {
        self.offset
    }
    fn step<TRange, FNext, Op>(
        &mut self,
        range: TRange,
        next: FNext,
        escape_bounds: &'a BTreeMap<usize, usize>,
        op: Op,
    )
        where
            TRange: SliceIndex<str, Output = str>,
            FNext: Fn(&mut CharIndices<'a>) -> Option<(usize, char)>,
            Op: Fn(&mut usize, usize),
    {
        let s = &self.s[range];
        if let Some(step) = next(&mut s
            .char_indices())
            .map(|(_, c)| match escape_bounds.get(&self.offset) {
                Some(&len) => len,
                None => c.len_utf8(),
            }) {
                op(&mut self.offset, step);
            } else {
                self.end = true;
        }
    }
}

impl<'a> Iterator for RunIndex<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end {
            return None;
        }
        let i = self.offset;
        self.step(
            self.offset..,
            CharIndices::next,
            self.left_escape_bounds,
            AddAssign::add_assign,
        );
        Some(i)
    }
}

impl<'a> DoubleEndedIterator for RunIndex<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end {
            return None;
        }
        let i = self.offset;
        self.step(
            ..self.offset,
            CharIndices::next_back,
            self.right_escape_bounds,
            SubAssign::sub_assign,
        );
        Some(i)
    }
}

pub struct RunIter<'a> {
    s: &'a str,
    init_left_off: usize,
    init_right_off: usize,
    left_escape_bounds: &'a BTreeMap<usize, usize>,
    right_escape_bounds: &'a BTreeMap<usize, usize>,
    left_off: RunIndex<'a>,
    right_off: RunIndex<'a>,
}

impl<'a> RunIter<'a> {
    pub fn range(&self) -> Range<usize> {
        self.left_off.peek()..self.right_off.peek()
    }
    pub fn get(&self) -> &'a str {
        &self.s[self.range()]
    }
}

impl<'a> Iterator for RunIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        Some(match (self.left_off.next(), self.right_off.next()) {
            (Some(left), Some(right)) => &self.s[left..right],
            (Some(_), None) => {
                self.left_off = RunIndex::new(self.s, 0, self.left_escape_bounds, self.right_escape_bounds);
                self.right_off = RunIndex::new(self.s, self.init_right_off, self.left_escape_bounds, self.right_escape_bounds);
                &self.s[self.left_off.next().unwrap()..self.right_off.next().unwrap()]
            } 
            _ => unreachable!(),
        })
    }
}

impl<'a> DoubleEndedIterator for RunIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(match (self.left_off.next_back(), self.right_off.next_back()) {
            (Some(left), Some(right)) => &self.s[left..right],
            (None, Some(_)) => {
                self.left_off = RunIndex::new(self.s, self.init_left_off, self.left_escape_bounds, self.right_escape_bounds);
                self.right_off = RunIndex::new(self.s, self.s.len(), self.left_escape_bounds, self.right_escape_bounds);
                &self.s[self.left_off.next_back().unwrap()..self.right_off.next_back().unwrap()]
            } 
            _ => unreachable!(),
        })
    }
}


#[cfg(test)]
mod tests {
    use anyhow::{Ok, Result};

    use super::RunningText;

    macro_rules! assert_text {
        ($it:expr, $($iter:literal),+) => {
            let mut it = $it;
            $(assert_eq!(it.next().unwrap(), $iter));+
        }
    }

    #[test]
    fn one_full_cycle() -> Result<()> {
        let text = RunningText::new::<&str>(

            "I am a running text|".to_owned(),
            12,
            true,
            &[],
        );
        assert_text!(
            text.iter(),
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
    fn one_full_cycle_backwards() -> Result<()> {
        let text = RunningText::new::<&str>(

            "I am a running text|".to_owned(),
            12,
            true,
            &[],
        );
        assert_text!(
            text.iter().rev(),
            "I am a runni",
            "|I am a runn",
            "t|I am a run",
            "xt|I am a ru",
            "ext|I am a r",
            "text|I am a ",
            " text|I am a",
            "g text|I am ",
            "ng text|I am",
            "ing text|I a",
            "ning text|I ",
            "nning text|I",
            "unning text|",
            "running text",
            " running tex",
            "a running te",
            " a running t",
            "m a running ",
            "am a running",
            " am a runnin",
            "I am a runni"
        );
        Ok(())
    }

    #[test]
    fn one_full_cycle_at() -> Result<()> {
        let text = RunningText::new::<&str>(

            "I am a running text|".to_owned(),
            12,
            true,
            &[],
        );
        assert_text!(
            text.iter_at(5),
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
            "I am a runni",
            " am a runnin",
            "am a running",
            "m a running ",
            " a running t",
            "a running te"
        );
        Ok(())
    }

    #[test]
    fn with_repeat() -> Result<()> {
        let text = RunningText::new::<&str>(
            "I am a running text|".to_owned(),
            25,
            true,
            &[],
        );
        assert_text!(
            text.iter(),
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
    fn with_repeat_backwards() -> Result<()> {
        let text = RunningText::new::<&str>(
            "I am a running text|".to_owned(),
            25,
            true,
            &[],
        );
        assert_text!(
            text.iter().rev(),
            "I am a running text|I am ",
            "|I am a running text|I am",
            "t|I am a running text|I a",
            "xt|I am a running text|I ",
            "ext|I am a running text|I",
            "text|I am a running text|",
            " text|I am a running text",
            "g text|I am a running tex",
            "ng text|I am a running te",
            "ing text|I am a running t",
            "ning text|I am a running ",
            "nning text|I am a running",
            "unning text|I am a runnin",
            "running text|I am a runni",
            " running text|I am a runn",
            "a running text|I am a run",
            " a running text|I am a ru",
            "m a running text|I am a r",
            "am a running text|I am a ",
            " am a running text|I am a",
            "I am a running text|I am "
        );
        Ok(())
    }

    #[test]
    fn special_chars() -> Result<()> {
        let text = RunningText::new::<&str>(
            "?#@!$%^^&*()".to_owned(),
            12,
            true,
            &[],
        );
        assert_text!(
            text.iter(),
            "?#@!$%^^&*()",
            "#@!$%^^&*()?",
            "@!$%^^&*()?#",
            "!$%^^&*()?#@",
            "$%^^&*()?#@!",
            "%^^&*()?#@!$",
            "^^&*()?#@!$%",
            "^&*()?#@!$%^",
            "&*()?#@!$%^^",
            "*()?#@!$%^^&",
            "()?#@!$%^^&*",
            ")?#@!$%^^&*(",
            "?#@!$%^^&*()",
            "#@!$%^^&*()?",
            "@!$%^^&*()?#",
            "!$%^^&*()?#@"
        );
        Ok(())
    }

    #[test]
    fn replacement() -> Result<()> {
        let text = RunningText::new(
            "?#@!$%^^&*()".to_owned(),
            12,
            true,
            &[
                ("&", "&amp"),
            ],
        );
        assert_text!(
            text.iter(),
            "?#@!$%^^&amp*()",
            "#@!$%^^&amp*()?",
            "@!$%^^&amp*()?#",
            "!$%^^&amp*()?#@",
            "$%^^&amp*()?#@!",
            "%^^&amp*()?#@!$",
            "^^&amp*()?#@!$%",
            "^&amp*()?#@!$%^",
            "&amp*()?#@!$%^^",
            "*()?#@!$%^^&amp",
            "()?#@!$%^^&amp*",
            ")?#@!$%^^&amp*(",
            "?#@!$%^^&amp*()",
            "#@!$%^^&amp*()?",
            "@!$%^^&amp*()?#",
            "!$%^^&amp*()?#@"
        );
        Ok(())
    }

    #[test]
    fn replacement_at() -> Result<()> {
        let text = RunningText::new(
            "?#@!$%^^&*()".to_owned(),
            12,
            true,
            &[
                ("&", "&amp"),
            ],
        );
        assert_text!(
            text.iter_at(10),
            "&amp*()?#@!$%^^",
            "*()?#@!$%^^&amp",
            "()?#@!$%^^&amp*",
            ")?#@!$%^^&amp*(",
            "?#@!$%^^&amp*()",
            "#@!$%^^&amp*()?",
            "@!$%^^&amp*()?#",
            "!$%^^&amp*()?#@",
            "$%^^&amp*()?#@!",
            "%^^&amp*()?#@!$",
            "^^&amp*()?#@!$%",
            "^&amp*()?#@!$%^",
            "&amp*()?#@!$%^^",
            "*()?#@!$%^^&amp",
            "()?#@!$%^^&amp*",
            ")?#@!$%^^&amp*("
        );
        Ok(())
    }

    #[test]
    fn replacement_backwards() -> Result<()> {
        let text = RunningText::new(
            "?#@!$%^^&*()".to_owned(),
            12,
            true,
            &[
                ("&", "&amp"),
            ],
        );
        assert_text!(
            text.iter().rev(),
            "?#@!$%^^&amp*()",
            ")?#@!$%^^&amp*(",
            "()?#@!$%^^&amp*",
            "*()?#@!$%^^&amp",
            "&amp*()?#@!$%^^",
            "^&amp*()?#@!$%^",
            "^^&amp*()?#@!$%",
            "%^^&amp*()?#@!$",
            "$%^^&amp*()?#@!",
            "!$%^^&amp*()?#@",
            "@!$%^^&amp*()?#",
            "#@!$%^^&amp*()?",
            "?#@!$%^^&amp*()"
        );
        Ok(())
    }

    #[test]
    fn replacement_backwards_at() -> Result<()> {
        let text = RunningText::new(
            "?#@!$%^^&*()".to_owned(),
            12,
            true,
            &[
                ("&", "&amp"),
            ],
        );
        assert_text!(
            text.iter_at(10).rev(),
            "&amp*()?#@!$%^^",
            "^&amp*()?#@!$%^",
            "^^&amp*()?#@!$%",
            "%^^&amp*()?#@!$",
            "$%^^&amp*()?#@!",
            "!$%^^&amp*()?#@",
            "@!$%^^&amp*()?#",
            "#@!$%^^&amp*()?",
            "?#@!$%^^&amp*()",
            ")?#@!$%^^&amp*(",
            "()?#@!$%^^&amp*",
            "*()?#@!$%^^&amp",
            "&amp*()?#@!$%^^",
            "^&amp*()?#@!$%^",
            "^^&amp*()?#@!$%"
        );
        Ok(())
    }

    #[test]
    fn without_repeat() -> Result<()> {
        let text = RunningText::new::<&str>(
            "a & b".to_owned(),
            5,
            false,
            &[],
        );
        assert_text!(text.iter(), "a & b", "a & b", "a & b", "a & b");
        Ok(())
    }

    #[test]
    fn without_repeat_backwards() -> Result<()> {
        let text = RunningText::new::<&str>(
            "a & b".to_owned(),
            5,
            false,
            &[],
        );
        assert_text!(text.iter().rev(), "a & b", "a & b", "a & b", "a & b");
        Ok(())
    }

    #[test]
    fn replacement_without_repeat() -> Result<()> {
        let text = RunningText::new(
            "a & b".to_owned(),
            5,
            false,
            &[("&", "&amp;")],
        );
        assert_text!(text.iter(), "a &amp; b", "a &amp; b", "a &amp; b", "a &amp; b");
        Ok(())
    }

    #[test]
    fn replacement_without_repeat_backwards() -> Result<()> {
        let text = RunningText::new(
            "a & b".to_owned(),
            5,
            false,
            &[("&", "&amp;")],
        );
        assert_text!(text.iter().rev(), "a &amp; b", "a &amp; b", "a &amp; b", "a &amp; b");
        Ok(())
    }
}
