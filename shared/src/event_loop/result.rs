#[derive(Debug)]
pub struct EventLoopResult {
    pub time: Option<u64>,
    pub fd: Option<EventLoopFdResult>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EventLoopFdResult {
    pub readable: bool,
    pub writable: bool,
    pub has_error: bool,
}
