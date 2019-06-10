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

            v.push(b'*');
            v.extend_from_slice(count_bytes);
            v.extend_from_slice(b"\r\n");

            $(insert_bytes_into_vec!(v, $item);)*;

            v
        }
    };
}

macro_rules! insert_bytes_into_vec {
    ($arg:ident, $str:expr) => {
        {
            let input_raw = $str;
            let input = input_raw.as_bytes();

            let length = input.len();
            let length_str_raw = length.to_string();
            let length_str = length_str_raw.as_bytes();

            $arg.reserve(1 + length_str.len() + 2 + length + 2);
            $arg.push(b'$');
            $arg.extend_from_slice(length_str);
            $arg.extend_from_slice(b"\r\n");
            $arg.extend_from_slice(input);
            $arg.extend_from_slice(b"\r\n");
        }
    };
}
