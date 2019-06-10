macro_rules! _replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! _count_tts {
    ($($tts:tt)*) => {0usize $(+ _replace_expr!($tts 1usize))*};
}

macro_rules! resp_bytes {
    ($($item:expr),*) => {
        {
            let count = _count_tts!($($item)*).to_string();
            let count_bytes = count.as_bytes();
            let mut v = Vec::with_capacity(3 + count_bytes.len());

            v.push('*' as u8);
            v.extend_from_slice(count_bytes);
            v.extend_from_slice("\r\n".as_bytes());

            $(_resp_bytes_each_impl!(v, $item);)*;

            v
        }
    };
}

macro_rules! _resp_bytes_each_impl {
    ($arg:ident, $str:expr) => {
        {
            let length = $str.len();
            let length_str_raw = length.to_string();
            let length_str = length_str_raw.as_bytes();
            $arg.reserve(1 + length_str.len() + 2 + length + 2);
            $arg.push('$' as u8);
            $arg.extend_from_slice(length_str);
            $arg.extend_from_slice("\r\n".as_bytes());
            $arg.extend_from_slice($str.as_bytes());
            $arg.extend_from_slice("\r\n".as_bytes());
        }
    };
}
