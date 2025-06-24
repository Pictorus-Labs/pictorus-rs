pub trait FlattenSlice<T> {
    fn as_flattened(&self) -> &[T];
    fn as_flattened_mut(&mut self) -> &mut [T];
}

impl<T, const N: usize> FlattenSlice<T> for [[T; N]] {
    fn as_flattened(&self) -> &[T] {
        // SAFETY: The slice is guaranteed to be valid and contiguous.
        unsafe { core::slice::from_raw_parts(self.as_ptr() as *const T, N * self.len()) }
    }

    fn as_flattened_mut(&mut self) -> &mut [T] {
        // SAFETY: The slice is guaranteed to be valid and contiguous.
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr() as *mut T, N * self.len()) }
    }
}

impl<T, const N: usize, const M: usize> FlattenSlice<T> for [[T; N]; M] {
    fn as_flattened(&self) -> &[T] {
        // SAFETY: The slice is guaranteed to be valid and contiguous.
        unsafe { core::slice::from_raw_parts(self.as_ptr() as *const T, N * self.len()) }
    }

    fn as_flattened_mut(&mut self) -> &mut [T] {
        // SAFETY: The slice is guaranteed to be valid and contiguous.
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr() as *mut T, N * self.len()) }
    }
}
