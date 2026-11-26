pub(crate) struct Logger;

impl Logger {
    pub(crate) fn init() {
        let _ = mpclipboard_shared::Logger::init();

        log::info!("info example");
        log::error!("error example");
    }
}
