use quinn_proto::VarInt;

#[derive(Clone, Copy)]
pub(crate) struct StreamId(u64);

impl From<quinn_proto::StreamId> for StreamId {
    fn from(id: quinn_proto::StreamId) -> Self {
        Self(VarInt::from(id).into_inner())
    }
}

impl From<StreamId> for quinn_proto::StreamId {
    fn from(id: StreamId) -> Self {
        let id = VarInt::from_u64(id.0)
            .unwrap_or_else(|_| unreachable!("h3wire only reports streams it got from quinn"));
        Self::from(id)
    }
}

impl From<h3wire::StreamId> for StreamId {
    fn from(id: h3wire::StreamId) -> Self {
        Self(id.0)
    }
}

impl From<StreamId> for h3wire::StreamId {
    fn from(id: StreamId) -> Self {
        Self(id.0)
    }
}
