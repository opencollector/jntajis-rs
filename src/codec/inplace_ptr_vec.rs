use std::ptr::NonNull;

use crate::array_vec::ArrayVecWithLen;

/// A vector of `NonNull<T>` that stores up to `CAP` pointers inline, spilling to the heap beyond that.
#[derive(Clone, Debug)]
pub enum InplacePtrVec<T: Clone, const CAP: usize> {
    Inplace(ArrayVecWithLen<NonNull<T>, CAP>),
    Heap(Vec<NonNull<T>>),
}

impl<T: Clone, const CAP: usize> Default for InplacePtrVec<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const CAP: usize> InplacePtrVec<T, CAP> {
    /// Creates an empty inline-mode vector.
    pub fn new() -> Self {
        InplacePtrVec::Inplace(ArrayVecWithLen::new())
    }

    /// Creates a vector with the given capacity hint; uses heap if `capacity > CAP`.
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= CAP {
            InplacePtrVec::Inplace(ArrayVecWithLen::new())
        } else {
            InplacePtrVec::Heap(Vec::with_capacity(capacity))
        }
    }

    /// Appends a pointer, spilling to the heap if inline capacity is exceeded.
    pub fn push(&mut self, value: NonNull<T>) {
        match self {
            InplacePtrVec::Inplace(vec) => {
                if vec.len() < CAP {
                    vec.push(value);
                } else {
                    let mut heap_vec = Vec::with_capacity(CAP);
                    heap_vec.extend(vec.iter().cloned());
                    heap_vec.push(value);
                    *self = InplacePtrVec::Heap(heap_vec);
                }
            }
            InplacePtrVec::Heap(vec) => vec.push(value),
        }
    }

    /// Returns the number of stored pointers.
    pub fn len(&self) -> usize {
        match self {
            InplacePtrVec::Inplace(vec) => vec.len(),
            InplacePtrVec::Heap(vec) => vec.len(),
        }
    }

    /// Returns `true` if the vector is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            InplacePtrVec::Inplace(vec) => vec.is_empty(),
            InplacePtrVec::Heap(vec) => vec.is_empty(),
        }
    }

    /// Returns the stored pointers as a slice.
    pub fn as_slice(&self) -> &[NonNull<T>] {
        match self {
            InplacePtrVec::Inplace(vec) => &vec[..],
            InplacePtrVec::Heap(vec) => vec.as_slice(),
        }
    }
}

/// Owned iterator over an [`InplacePtrVec`].
pub struct InplacePtrVecIntoIter<T: Clone, const CAP: usize>(InplacePtrVec<T, CAP>, usize);

impl<T: Clone, const CAP: usize> Iterator for InplacePtrVecIntoIter<T, CAP> {
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            InplacePtrVec::Inplace(vec) => {
                if self.1 < vec.len() {
                    let item = vec[self.1];
                    self.1 += 1;
                    Some(item)
                } else {
                    None
                }
            }
            InplacePtrVec::Heap(vec) => {
                if self.1 < vec.len() {
                    let item = vec[self.1];
                    self.1 += 1;
                    Some(item)
                } else {
                    None
                }
            }
        }
    }
}

/// Borrowing iterator over an [`InplacePtrVec`].
pub struct InplacePtrVecPtrIter<'a, T: Clone + 'a, const CAP: usize>(
    &'a InplacePtrVec<T, CAP>,
    usize,
);

impl<'a, T: Clone + 'a, const CAP: usize> Iterator for InplacePtrVecPtrIter<'a, T, CAP> {
    type Item = &'a NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            InplacePtrVec::Inplace(vec) => {
                if self.1 < vec.len() {
                    let item = &vec[self.1];
                    self.1 += 1;
                    Some(item)
                } else {
                    None
                }
            }
            InplacePtrVec::Heap(vec) => {
                if self.1 < vec.len() {
                    let item = &vec[self.1];
                    self.1 += 1;
                    Some(item)
                } else {
                    None
                }
            }
        }
    }
}

impl<T: Clone, const CAP: usize> IntoIterator for InplacePtrVec<T, CAP> {
    type Item = NonNull<T>;
    type IntoIter = InplacePtrVecIntoIter<T, CAP>;

    fn into_iter(self) -> Self::IntoIter {
        InplacePtrVecIntoIter(self, 0)
    }
}

impl<'a, T: Clone + 'a, const CAP: usize> IntoIterator for &'a InplacePtrVec<T, CAP> {
    type Item = &'a NonNull<T>;
    type IntoIter = InplacePtrVecPtrIter<'a, T, CAP>;

    fn into_iter(self) -> Self::IntoIter {
        InplacePtrVecPtrIter(self, 0)
    }
}

impl<T: Clone, const CAP: usize> FromIterator<NonNull<T>> for InplacePtrVec<T, CAP> {
    fn from_iter<I: IntoIterator<Item = NonNull<T>>>(iter: I) -> Self {
        let mut vec = InplacePtrVec::new();
        for item in iter {
            vec.push(item);
        }
        vec
    }
}

impl<T: Clone, const CAP: usize> From<Vec<NonNull<T>>> for InplacePtrVec<T, CAP> {
    fn from(vec: Vec<NonNull<T>>) -> Self {
        if vec.len() <= CAP {
            InplacePtrVec::Inplace(ArrayVecWithLen::try_from(vec).unwrap())
        } else {
            InplacePtrVec::Heap(vec)
        }
    }
}

impl<T: Clone, const CAP: usize> From<&[NonNull<T>]> for InplacePtrVec<T, CAP> {
    fn from(slice: &[NonNull<T>]) -> Self {
        if slice.len() <= CAP {
            InplacePtrVec::Inplace(ArrayVecWithLen::try_from(slice).unwrap())
        } else {
            InplacePtrVec::Heap(Vec::from(slice))
        }
    }
}

impl<T: Clone, const CAP: usize> std::ops::Deref for InplacePtrVec<T, CAP> {
    type Target = [NonNull<T>];

    fn deref(&self) -> &Self::Target {
        match self {
            InplacePtrVec::Inplace(vec) => vec.as_ref(),
            InplacePtrVec::Heap(vec) => vec.as_ref(),
        }
    }
}

impl<T: Clone, const CAP: usize> std::ops::DerefMut for InplacePtrVec<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            InplacePtrVec::Inplace(vec) => vec.as_mut(),
            InplacePtrVec::Heap(vec) => vec.as_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::NonNull;

    #[test]
    fn test_inplace_ptr_vec() {
        let mut vec = InplacePtrVec::<u32, 4>::new();
        assert!(vec.is_empty());
        assert_eq!(vec.len(), 0);
        for i in 0..5 {
            vec.push(NonNull::new(Box::into_raw(Box::new(i))).unwrap());
        }
        assert!(!vec.is_empty());
        assert_eq!(vec.len(), 5);
        let mut iter = vec.into_iter();
        for i in 0..5 {
            assert_eq!(unsafe { *iter.next().unwrap().as_ref() }, i);
        }
        assert!(iter.next().is_none());
    }
}
