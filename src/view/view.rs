pub struct View<'a, T> {
    data: &'a [T]
}

pub struct ViewMut<'a, T> {
    data: &'a mut [T]
}

impl<'a, T> View<'a, T> {
    pub fn new(data: &'a [T]) -> Self { View { data } }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn as_slice(&self) -> &'a [T] { self.data }
}