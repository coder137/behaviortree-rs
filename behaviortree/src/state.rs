use behaviortree_common::Status;

#[derive(Clone)]
pub enum State {
    NoChild(&'static str, tokio::sync::watch::Receiver<Option<Status>>),
    SingleChild(
        &'static str,
        tokio::sync::watch::Receiver<Option<Status>>,
        std::rc::Rc<State>,
    ),
    MultipleChildren(
        &'static str,
        tokio::sync::watch::Receiver<Option<Status>>,
        std::rc::Rc<[State]>,
    ),
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoChild(name, status) => f
                .debug_tuple("NoChild")
                .field(name)
                .field(&(*status.borrow()))
                .finish(),
            Self::SingleChild(name, status, state) => f
                .debug_tuple("SingleChild")
                .field(name)
                .field(&(*status.borrow()))
                .field(state)
                .finish(),
            Self::MultipleChildren(name, status, states) => f
                .debug_tuple("MultipleChildren")
                .field(name)
                .field(&(*status.borrow()))
                .field(states)
                .finish(),
        }
    }
}
