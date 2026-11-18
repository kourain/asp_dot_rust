pub struct ReadOnly<T>(T);

impl<T> ReadOnly<T> {
    pub fn new(value: T) -> Self {
        ReadOnly(value)
    }
}

impl<T> std::ops::Deref for ReadOnly<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
