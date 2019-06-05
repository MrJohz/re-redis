pub(crate) fn number_length(mut number: u128) -> usize {
    if number == 0 {
        return 1;
    }

    let mut base = 0;

    while number > 0 {
        number = number / 10;
        base += 1;
    }

    base
}

pub(crate) fn validate_key(key: impl Into<String>) -> String {
    let key = key.into();
    if key.len() > 512 * 1000 * 1000 {
        // 512 MB, roughly
        panic!("key is too large (over 512 MB)");
    }
    key
}

#[cfg(test)]
mod number_length_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[test]
    fn works_correctly_for_0() {
        assert_eq!(number_length(0), 1);
    }

    #[test]
    fn works_correctly_for_1() {
        assert_eq!(number_length(1), 1);
    }

    #[test]
    fn works_correctly_for_10() {
        assert_eq!(number_length(10), 2);
    }

    #[test]
    fn works_correctly_for_34() {
        assert_eq!(number_length(34), 2);
    }

    #[quickcheck]
    fn has_the_correct_length(number: u128) {
        assert_eq!(number.to_string().len(), number_length(number));
    }
}
