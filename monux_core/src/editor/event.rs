#[derive(Debug, Clone)]
pub enum Event {
    Line(String),
    Message(String),
}
