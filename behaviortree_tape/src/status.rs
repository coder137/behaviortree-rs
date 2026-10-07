#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Success,
    Failure,
    Running,
}

impl From<bool> for Status {
    fn from(value: bool) -> Self {
        if value {
            Status::Success
        } else {
            Status::Failure
        }
    }
}
