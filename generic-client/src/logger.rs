pub struct Logger;

impl Logger {
    pub(crate) fn init() {
        #[cfg(target_os = "android")]
        {
            use android_logger::Config;
            use log::LevelFilter;

            #[cfg(debug_assertions)]
            let max_level = LevelFilter::Trace;
            #[cfg(not(debug_assertions))]
            let max_level = LevelFilter::Error;

            android_logger::init_once(Config::default().with_tag("RUST").with_max_level(max_level));
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        env_logger::init();

        log::info!("info example");
        log::error!("error example");
    }
}
