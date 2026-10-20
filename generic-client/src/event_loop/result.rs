use mpclipboard_shared::EventLoopFdResult;

#[derive(Debug)]
pub(crate) struct EventLoopResult {
    pub(crate) ticked: bool,
    pub(crate) fd: Option<EventLoopFdResult>,
}
