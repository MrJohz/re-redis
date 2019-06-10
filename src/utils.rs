pub(crate) fn validate_key(key: impl Into<String>) -> String {
    let key = key.into();
    if key.len() > 512 * 1000 * 1000 {
        // 512 MB, roughly
        panic!("key is too large (over 512 MB)");
    }
    key
}
