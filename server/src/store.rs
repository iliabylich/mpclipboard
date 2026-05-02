use clip::Clip;

pub(crate) struct Store {
    clip: Option<Clip>,
}

impl Store {
    pub(crate) const fn empty() -> Self {
        Self { clip: None }
    }

    pub(crate) fn current(&self) -> Option<Clip> {
        self.clip.clone()
    }

    #[must_use]
    pub(crate) fn add(&mut self, clip: &Clip) -> bool {
        let do_update = self.clip.is_none()
            || self
                .clip
                .as_ref()
                .is_some_and(|current| clip.newer_than(current));

        if do_update {
            self.clip = Some(clip.clone());
        }

        do_update
    }
}
