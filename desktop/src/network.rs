#[derive(Debug, Default)]
pub struct Network {}

impl Network {
    pub fn new(port: u16) -> Self {
        Self {
            ..Default::default()
        }
    }
}
