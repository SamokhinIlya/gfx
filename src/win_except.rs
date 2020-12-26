pub fn win_except(result: impl WinResult, context: impl AsRef<str>) {
    use winapi::um::errhandlingapi::GetLastError;

    if result.is_error() {
        panic!("{}.\nLast error code: {}", context.as_ref(), unsafe { GetLastError() });
    }
}

pub trait WinResult {
    fn is_error(&self) -> bool;
}

impl<T> WinResult for *const T {
    fn is_error(&self) -> bool {
        self.is_null()
    }
}

impl<T> WinResult for *mut T {
    fn is_error(&self) -> bool {
        self.is_null()
    }
}

macro_rules! impl_win_result_for_number {
    ($($t:ty),+ $(,)?) => {
        $(
            impl WinResult for $t {
                fn is_error(&self) -> bool {
                    *self == 0
                }
            }
        )+
    }
}

impl_win_result_for_number!(
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
);