#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Success,
    Failure,
    Running,
}

impl From<bool> for Status {
    fn from(status: bool) -> Self {
        match status {
            true => Status::Success,
            false => Status::Failure,
        }
    }
}
