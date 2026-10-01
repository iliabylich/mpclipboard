use crate::Connectivity;

#[derive(Debug)]
#[must_use]
pub struct Output {
    pub connectivity: Option<Connectivity>,
    pub text: Option<String>,
}
