pub(crate) trait MarbleParse {
    type Input;

    fn parse(c: Self::Input) -> Self;
}

impl<T> MarbleParse for Vec<T>
where
    T: MarbleParse<Input = char>,
{
    type Input = &'static str;

    fn parse(s: &str) -> Self {
        assert!(s.is_ascii(), "non-ascii marble: {s:?}");

        assert!(
            s.len() <= 100,
            "marbles are limited to 100 frames (10s), so that now() fits in a single digit, got {}",
            s.len()
        );

        s.chars().map(T::parse).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Dummy;

    #[cfg(test)]
    impl MarbleParse for Dummy {
        type Input = char;

        fn parse(_: Self::Input) -> Self {
            Self
        }
    }

    const TOO_LONG: &str = match core::str::from_utf8(&[b'-'; 101]) {
        Ok(s) => s,
        Err(_) => panic!(),
    };

    #[test]
    #[should_panic(expected = "marbles are limited to 100 frames")]
    fn test_parse_too_long() {
        Vec::<Dummy>::parse(TOO_LONG);
    }

    #[test]
    #[should_panic(expected = "non-ascii")]
    fn test_parse_non_ascii() {
        Vec::<Dummy>::parse("😭");
    }
}
