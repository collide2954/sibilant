use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration<D>(i64, PhantomData<D>);

impl<D> Duration<D> {
    pub(super) const fn new(value: i64) -> Self {
        Self(value, PhantomData)
    }

    pub(super) const fn value(self) -> i64 {
        self.0
    }
}
