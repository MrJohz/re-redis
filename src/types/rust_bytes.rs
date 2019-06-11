use std::borrow::Cow;
use std::ops::Deref;

// TODO: provide a debug implementation that hides the implementation details
//   and also converts to a string where possible

#[derive(Debug, PartialEq, Eq)]
pub struct RBytes<'a>(Cow<'a, [u8]>);

impl<'a> RBytes<'a> {
    pub(crate) fn as_bytes(&'a self) -> &'a [u8] {
        self.0.deref()
    }
}

impl<'a> From<String> for RBytes<'a> {
    fn from(other: String) -> Self {
        RBytes(Cow::from(other.into_bytes()))
    }
}

impl<'a> From<&'a str> for RBytes<'a> {
    fn from(other: &'a str) -> Self {
        RBytes(Cow::from(other.as_bytes()))
    }
}

macro_rules! into_bytes_for_integers {
    ($kind:ty) => {
        impl<'a> From<$kind> for RBytes<'a> {
            fn from(other: $kind) -> Self {
                RBytes(Cow::from(other.to_string().into_bytes()))
            }
        }
    };
}

into_bytes_for_integers!(isize);
into_bytes_for_integers!(i64);
into_bytes_for_integers!(i32);
into_bytes_for_integers!(i16);
into_bytes_for_integers!(i8);

into_bytes_for_integers!(usize);
into_bytes_for_integers!(u64);
into_bytes_for_integers!(u32);
into_bytes_for_integers!(u16);
into_bytes_for_integers!(u8);

into_bytes_for_integers!(f64);
into_bytes_for_integers!(f32);

into_bytes_for_integers!(bool); // not really integers - shh! don't tell anyone!

impl<'a> From<&'a [u8]> for RBytes<'a> {
    fn from(other: &'a [u8]) -> Self {
        RBytes(Cow::from(other))
    }
}

impl<'a> From<Vec<u8>> for RBytes<'a> {
    fn from(other: Vec<u8>) -> Self {
        RBytes(Cow::from(other))
    }
}
