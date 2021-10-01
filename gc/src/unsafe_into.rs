pub trait UnsafeInto<T> {
    #[allow(clippy::missing_safety_doc)]
    unsafe fn unsafe_into(self) -> T;
}

impl<T> UnsafeInto<T> for T {
    unsafe fn unsafe_into(self) -> T {
        self
    }
}
