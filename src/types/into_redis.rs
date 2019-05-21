// TODO: we should probably just borrow the existing Into trait for this
pub trait IntoRedisValue {
    fn convert(self) -> String;
}

macro_rules! create_into_redis_impl {
    ($kind:ty, $name:ident => $conversion:block) => {
        impl IntoRedisValue for $kind {
            fn convert(self) -> String {
                let $name = self;
                $conversion
            }
        }
    };
    ($kind:ty, $name:ident => $conversion:expr) => {
        impl IntoRedisValue for $kind {
            fn convert(self) -> String {
                let $name = self;
                $conversion
            }
        }
    };
    ($kind:ty, default) => {
        impl IntoRedisValue for $kind {
            fn convert(self) -> String {
                self.to_string()
            }
        }
    };
}

create_into_redis_impl! {isize, default}
create_into_redis_impl! {i64, default}
create_into_redis_impl! {i32, default}
create_into_redis_impl! {i16, default}
create_into_redis_impl! {i8, default}

create_into_redis_impl! {usize, default}
create_into_redis_impl! {u64, default}
create_into_redis_impl! {u32, default}
create_into_redis_impl! {u16, default}
create_into_redis_impl! {u8, default}

create_into_redis_impl! {f64, default}
create_into_redis_impl! {f32, default}

create_into_redis_impl! {bool, default}
create_into_redis_impl! {char, default}

create_into_redis_impl! {&str, default}
create_into_redis_impl! {String, input => input}
