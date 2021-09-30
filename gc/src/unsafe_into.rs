pub trait UnsafeInto<T> {
    unsafe fn unsafe_into(self) -> T;
}

impl<T> UnsafeInto<T> for T {
    unsafe fn unsafe_into(self) -> T {
        self
    }
}
