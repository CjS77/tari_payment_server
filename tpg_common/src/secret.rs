use std::{
    fmt,
    fmt::{Debug, Display},
};

#[derive(Clone, Default)]
pub struct Secret<T>
where T: Clone + Default
{
    value: T,
}

impl<T: Clone + Default> Secret<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn reveal(&self) -> &T {
        &self.value
    }
}

impl<T: Clone + Default> Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("****")
    }
}

impl<T: Clone + Default> Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("****")
    }
}
