use mpclipboard_shared::EventLoopFdResult;

#[derive(Debug)]
pub(crate) struct EventLoopResult {
    pub(crate) time: Option<u64>,
    pub(crate) fd: Option<EventLoopFdResult>,
}
