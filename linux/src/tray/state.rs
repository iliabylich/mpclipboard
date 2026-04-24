use crate::tray::{buffer::Buffer, line::Line};
use mpclipboard_generic_client::Connectivity;
use tokio_util::sync::CancellationToken;

pub(crate) struct TrayState {
    pub(crate) connectivity: Connectivity,
    pub(crate) buffer: Buffer<5, Line>,
    token: CancellationToken,
}

impl ksni::Tray for TrayState {
    fn id(&self) -> String {
        "mpclipboard".to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        const GREEN: &[u8] = include_bytes!("../../assets/green-32x32.rgba");
        const RED: &[u8] = include_bytes!("../../assets/red-32x32.rgba");
        let bytes = match self.connectivity {
            Connectivity::Connecting => RED,
            Connectivity::Connected => GREEN,
            Connectivity::Disconnected => RED,
        };

        vec![ksni::Icon {
            width: 32,
            height: 32,
            data: bytes.to_vec(),
        }]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        self.buffer
            .iter()
            .map(MenuItem::from)
            .chain([
                MenuItem::Separator,
                MenuItem::Standard(StandardItem {
                    label: "Quit".to_string(),
                    activate: Box::new({
                        let token = self.token.clone();
                        move |_| token.cancel()
                    }),
                    ..Default::default()
                }),
            ])
            .collect()
    }
}

impl TrayState {
    pub(crate) fn new(token: CancellationToken) -> Self {
        Self {
            connectivity: Connectivity::Disconnected,
            buffer: Buffer::new(),
            token,
        }
    }
}
