/// Platform-specific implementation of the logger.
pub(crate) struct Logger;

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

            android_logger::init_once(
                Config::default()
                    .with_tag("RUST")
                    .with_max_level(max_level),
            );
        }

        #[cfg(not(target_os = "android"))]
        env_logger::Builder::from_default_env()
            .format_target(false)
            .write_style(env_logger::WriteStyle::Always)
            .init();
    }
}
