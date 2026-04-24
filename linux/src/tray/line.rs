use crate::tray::TrayState;

pub(crate) enum Line {
    Received(String),
    Sent(String),
}

impl From<&Line> for ksni::menu::MenuItem<TrayState> {
    fn from(event: &Line) -> Self {
        let label = match event {
            Line::Received(text) => format!("R {text}"),
            Line::Sent(text) => format!("S {text}"),
        };
        Self::Standard(ksni::menu::StandardItem {
            label,
            enabled: false,
            ..Default::default()
        })
    }
}
