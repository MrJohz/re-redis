use core::fmt::Write;
use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;

#[derive(PartialEq, Eq)]
pub struct RBytes<'a>(Cow<'a, [u8]>);

impl<'a> fmt::Debug for RBytes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_str("RBytes(")
            .and_then(|_| match String::from_utf8(self.0.to_vec()) {
                Ok(text) => f.write_char('b').and_then(|_| text.fmt(f)),
                Err(_) => self.0.fmt(f),
            })
            .and_then(|_| f.write_str(")"))
    }
}

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

macro_rules! impl_bytes_for_arrays {
    ($($N:expr)+) => {
        $(
            impl<'a> From<&'a [u8; $N]> for RBytes<'a> {
                fn from(other: &'a [u8; $N]) -> Self {
                    RBytes(Cow::from(other.as_ref()))
                }
            }
        )+
    }
}

// Okay, for this *alone*, macros are worth their weight in gold...
impl_bytes_for_arrays!(
    1 2 3 4 5 6 7 8 9 10
    11 12 13 14 15 16 17 18 19 20
    21 22 23 24 25 26 27 28 29 30
    31 32
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_possible_to_create_a_bytes_value_from_a_static_array() {
        let bytes = RBytes::from(b"this is a bytes array");

        assert_eq!(RBytes::from("this is a bytes array"), bytes);
    }

    #[test]
    fn debug_impl_returns_a_string_if_the_string_can_be_displayed() {
        let bytes = RBytes::from("test this all works");

        assert_eq!("RBytes(b\"test this all works\")", format!("{:?}", bytes));
    }

    #[test]
    fn debug_impl_returns_a_bytes_array_if_the_string_cannot_be_displayed() {
        let bytes = RBytes::from(b"\xFF\xF9\x00".as_ref());

        assert_eq!("RBytes([255, 249, 0])", format!("{:?}", bytes));
    }
}
