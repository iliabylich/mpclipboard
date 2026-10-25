mod busy;
pub(crate) use busy::BusyAction;

mod input;
pub(crate) use input::InputAction;

mod set_deadline;
pub(crate) use set_deadline::SetDeadlineAction;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Actions {
    pub(crate) set_deadline: Vec<SetDeadlineAction>,
    pub(crate) input: Vec<InputAction>,
    pub(crate) busy: Vec<BusyAction>,
}
