pub(crate) trait MarbleRender {
    type Output;

    fn render(&self) -> Self::Output;
}

impl<T> MarbleRender for Vec<T>
where
    T: MarbleRender<Output = char>,
{
    type Output = String;

    fn render(&self) -> Self::Output {
        self.iter().map(T::render).collect()
    }
}
