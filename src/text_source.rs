pub trait TextSource {
    fn get(&mut self) -> anyhow::Result<String>;
    fn get_if_changed(&mut self) -> Option<anyhow::Result<String>>;
}

impl Iterator for dyn TextSource {
    type Item = anyhow::Result<String>;
    fn next(&mut self) -> Option<Self::Item> {
        self.get_if_changed()
    }
}

impl TextSource for String {
    fn get(&mut self) -> anyhow::Result<String> {
        Ok(self.clone())
    }
    fn get_if_changed(&mut self) -> Option<anyhow::Result<String>> {
        None
    }
}

