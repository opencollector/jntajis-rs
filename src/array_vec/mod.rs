//! Stack-allocated vector types with sentinel-based and length-based variants.

use std::mem::MaybeUninit;

pub mod invalid_value;

use invalid_value::{DefaultValueValidity, ValueValidity};
use serde::{Deserialize, Serialize};

/// Iterator over references to valid elements in an [`ArrayVec`].
pub struct PtrIter<
    'a,
    T: Copy + PartialEq,
    const CAP: usize,
    V: ValueValidity<Target = T> = DefaultValueValidity<T>,
> {
    array: &'a [T; CAP],
    index: usize,
    _validator: std::marker::PhantomData<V>,
}

impl<'a, T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>>
    PtrIter<'a, T, CAP, V>
{
    /// Creates a new iterator over the given array.
    #[inline]
    pub fn new(array: &'a [T; CAP]) -> Self {
        Self {
            array,
            index: 0,
            _validator: std::marker::PhantomData,
        }
    }
}

impl<'a, T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> Iterator
    for PtrIter<'a, T, CAP, V>
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= CAP {
            return None;
        }
        let item = &self.array[self.index];
        if !V::is_valid(item) {
            return None;
        }
        self.index += 1;
        Some(item)
    }
}

/// Fixed-capacity vector backed by an array, using a sentinel value to mark empty slots.
///
/// Unlike [`ArrayVecWithLen`], this type does not store an explicit length. Instead, it
/// uses a [`ValueValidity`] strategy to distinguish occupied slots from empty ones.
/// This makes it suitable for types that have a natural "invalid" value (e.g., `u32::MAX`).
#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ArrayVec<
    T: Copy + PartialEq,
    const CAP: usize,
    V: ValueValidity<Target = T> = DefaultValueValidity<T>,
> {
    array: [T; CAP],
    _validator: std::marker::PhantomData<V>,
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> ArrayVec<T, CAP, V> {
    /// Creates an empty `ArrayVec` filled with invalid sentinel values.
    #[inline]
    pub fn new() -> Self {
        Self {
            array: [V::invalid_value(); CAP],
            _validator: std::marker::PhantomData,
        }
    }

    /// Creates an `ArrayVec` from a fixed-size array. Returns an error if `CAP2 > CAP`.
    #[inline]
    pub fn from_array<const CAP2: usize>(
        array: &[T; CAP2],
    ) -> Result<Self, ArrayVecConstructionError> {
        if CAP2 > CAP {
            return Err(ArrayVecConstructionError(format!(
                "too many elements for ArrayVec: {}",
                array.len()
            )));
        }
        let mut self_ = Self::new();
        self_.array[..CAP2].copy_from_slice(array);
        Ok(self_)
    }

    /// Creates an `ArrayVec` from a slice. Returns an error if the slice exceeds capacity.
    #[inline]
    pub fn from_slice(slice: &[T]) -> Result<Self, ArrayVecConstructionError> {
        if slice.len() > CAP {
            return Err(ArrayVecConstructionError(format!(
                "too many elements for ArrayVec: {}",
                slice.len()
            )));
        }
        let mut array = [V::invalid_value(); CAP];
        array[..slice.len()].copy_from_slice(slice);
        Ok(Self {
            array,
            _validator: std::marker::PhantomData,
        })
    }

    /// Appends a value to the first invalid slot. Panics if all slots are occupied.
    #[inline]
    pub fn push(&mut self, value: T) {
        let mut i = 0;
        while i < CAP {
            if !V::is_valid(&self.array[i]) {
                self.array[i] = value;
                return;
            }
            i += 1;
        }
        panic!(
            "ArrayVec overflow: tried to insert more than {} elements",
            CAP
        );
    }

    /// Returns the number of valid elements, scanning backwards from the end.
    #[inline]
    pub fn len(&self) -> usize {
        let mut i = self.array.len();
        while i > 0 {
            i -= 1;
            if V::is_valid(&self.array[i]) {
                i += 1;
                break;
            }
        }
        i
    }

    /// Returns `true` if there are no valid elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !V::is_valid(&self.array[0])
    }

    /// Returns an iterator over references to valid elements.
    #[inline]
    pub fn iter(&self) -> PtrIter<'_, T, CAP, V> {
        PtrIter {
            array: &self.array,
            index: 0,
            _validator: std::marker::PhantomData,
        }
    }

    /// Returns a reference to the element at `index`, or `None` if invalid or out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < CAP && V::is_valid(&self.array[index]) {
            Some(&self.array[index])
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element at `index`, or `None` if invalid or out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < CAP && V::is_valid(&self.array[index]) {
            Some(&mut self.array[index])
        } else {
            None
        }
    }

    /// Returns a slice of the valid elements.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        let len = self.len();
        &self.array[..len]
    }

    /// Returns a mutable slice of the valid elements.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.array[..len]
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> Default
    for ArrayVec<T, CAP, V>
{
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned when constructing an array-backed vector with too many elements.
#[derive(Clone, Debug)]
pub struct ArrayVecConstructionError(String);

impl std::fmt::Display for ArrayVecConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ArrayVecConstructionError {}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> FromIterator<T>
    for ArrayVec<T, CAP, V>
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = [V::invalid_value(); CAP];
        for (i, item) in iter.into_iter().enumerate() {
            if i >= CAP {
                panic!(
                    "ArrayVec overflow: tried to insert more than {} elements",
                    CAP
                );
            }
            array[i] = item;
        }
        Self {
            array,
            _validator: std::marker::PhantomData,
        }
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> IntoIterator
    for ArrayVec<T, CAP, V>
{
    type Item = T;
    type IntoIter = std::iter::Take<std::array::IntoIter<T, CAP>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.array.into_iter().take(self.len())
    }
}

impl<'a, T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> IntoIterator
    for &'a ArrayVec<T, CAP, V>
{
    type Item = &'a T;
    type IntoIter = PtrIter<'a, T, CAP, V>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> TryFrom<Vec<T>>
    for ArrayVec<T, CAP, V>
{
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        Self::from_slice(&value[..])
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> TryFrom<&[T]>
    for ArrayVec<T, CAP, V>
{
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: &[T]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>, const CAP2: usize>
    TryFrom<[T; CAP2]> for ArrayVec<T, CAP, V>
{
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: [T; CAP2]) -> Result<Self, Self::Error> {
        Self::from_array(&value)
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>, const CAP2: usize>
    TryFrom<&[T; CAP2]> for ArrayVec<T, CAP, V>
{
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: &[T; CAP2]) -> Result<Self, Self::Error> {
        Self::from_array(value)
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> std::ops::Deref
    for ArrayVec<T, CAP, V>
{
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Copy + PartialEq, const CAP: usize, V: ValueValidity<Target = T>> std::ops::DerefMut
    for ArrayVec<T, CAP, V>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<'de, T: Copy + PartialEq + Deserialize<'de>, const CAP: usize, V: ValueValidity<Target = T>>
    Deserialize<'de> for ArrayVec<T, CAP, V>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec: Vec<T> = Deserialize::deserialize(deserializer)?;
        Self::try_from(vec).map_err(serde::de::Error::custom)
    }
}

impl<T: Copy + PartialEq + Serialize, const CAP: usize, V: ValueValidity<Target = T>> Serialize
    for ArrayVec<T, CAP, V>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}

/// Fixed-capacity vector backed by an array, with an explicit length field.
///
/// Unlike [`ArrayVec`], this type stores an explicit `len` and uses `MaybeUninit`
/// for uninitialized slots. This supports element types that lack a sentinel value.
#[derive(Clone, Debug)]
pub struct ArrayVecWithLen<T: Copy, const CAP: usize> {
    len: usize,
    elts: [MaybeUninit<T>; CAP],
}

impl<T: Copy, const CAP: usize> ArrayVecWithLen<T, CAP> {
    /// Creates an empty `ArrayVecWithLen`.
    #[inline]
    pub fn new() -> Self {
        Self {
            len: 0,
            elts: [MaybeUninit::<T>::uninit(); CAP],
        }
    }

    /// Creates an `ArrayVecWithLen` from a slice. Returns an error if the slice exceeds capacity.
    #[inline]
    pub fn from_slice(slice: &[T]) -> Result<Self, ArrayVecConstructionError> {
        if slice.len() > CAP {
            return Err(ArrayVecConstructionError(format!(
                "too many elements for ArrayVecWithLen: {}",
                slice.len()
            )));
        }
        let mut elts = [MaybeUninit::<T>::uninit(); CAP];
        // SAFETY: MaybeUninit<T> has the same layout as T, and slice.len() <= CAP is checked above.
        unsafe {
            std::ptr::copy_nonoverlapping(
                slice.as_ptr(),
                elts.as_mut_ptr().cast::<T>(),
                slice.len(),
            );
        }
        Ok(Self {
            len: slice.len(),
            elts,
        })
    }

    /// Appends a value. Panics if capacity is exceeded.
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len >= CAP {
            panic!(
                "ArrayVecWithLen overflow: tried to insert more than {} elements",
                CAP
            );
        }
        self.elts[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    /// Returns the number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over references to the elements.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Returns a reference to the element at `index`, or `None` if out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            // SAFETY: elements 0..self.len are initialized by construction.
            Some(unsafe { self.elts[index].assume_init_ref() })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element at `index`, or `None` if out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.len {
            // SAFETY: elements 0..self.len are initialized by construction.
            Some(unsafe { self.elts[index].assume_init_mut() })
        } else {
            None
        }
    }

    /// Returns a slice of the initialized elements.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: elements 0..self.len are initialized by construction.
        unsafe { std::slice::from_raw_parts(self.elts.as_ptr().cast::<T>(), self.len) }
    }

    /// Returns a mutable slice of the initialized elements.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: elements 0..self.len are initialized by construction.
        unsafe { std::slice::from_raw_parts_mut(self.elts.as_mut_ptr().cast::<T>(), self.len) }
    }
}

impl<T: Copy, const CAP: usize> Default for ArrayVecWithLen<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const CAP: usize> FromIterator<T> for ArrayVecWithLen<T, CAP> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = [MaybeUninit::<T>::uninit(); CAP];
        let mut i = 0;
        for item in iter {
            if i >= CAP {
                panic!(
                    "ArrayVecWithLen overflow: tried to insert more than {} elements",
                    CAP
                );
            }
            array[i] = MaybeUninit::new(item);
            i += 1;
        }
        Self {
            len: i,
            elts: array,
        }
    }
}

impl<T: Copy, const CAP: usize> IntoIterator for ArrayVecWithLen<T, CAP> {
    type Item = T;
    type IntoIter = std::iter::Map<
        std::iter::Take<std::array::IntoIter<MaybeUninit<T>, CAP>>,
        fn(MaybeUninit<T>) -> T,
    >;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        #[inline]
        fn unwrap_maybe_uninit<T>(mu: MaybeUninit<T>) -> T {
            unsafe { mu.assume_init() }
        }
        self.elts
            .into_iter()
            .take(self.len())
            .map(unwrap_maybe_uninit)
    }
}

impl<'a, T: Copy, const CAP: usize> IntoIterator for &'a ArrayVecWithLen<T, CAP> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Copy, const CAP: usize> TryFrom<Vec<T>> for ArrayVecWithLen<T, CAP> {
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        (&value[..]).try_into()
    }
}

impl<T: Copy, const CAP: usize> TryFrom<&[T]> for ArrayVecWithLen<T, CAP> {
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: &[T]) -> Result<Self, Self::Error> {
        Self::from_slice(value)
    }
}

impl<T: Copy, const CAP: usize> std::ops::Deref for ArrayVecWithLen<T, CAP> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T: Copy, const CAP: usize> std::ops::DerefMut for ArrayVecWithLen<T, CAP> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<'de, T: Copy + Deserialize<'de>, const CAP: usize> Deserialize<'de>
    for ArrayVecWithLen<T, CAP>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec: Vec<T> = Deserialize::deserialize(deserializer)?;
        Self::try_from(vec).map_err(serde::de::Error::custom)
    }
}

impl<T: Copy + Serialize, const CAP: usize> Serialize for ArrayVecWithLen<T, CAP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_slice().serialize(serializer)
    }
}

/// A compact byte vector that packs up to 4 bytes into a single `u32`.
///
/// Bytes are stored in big-endian order within the `u32`, and the length
/// is derived from the number of leading zero bytes.
#[derive(Clone, Copy)]
pub struct PackedU8Vec(u32);

impl Default for PackedU8Vec {
    fn default() -> Self {
        Self::new()
    }
}

impl PackedU8Vec {
    const MAX_LEN: usize = std::mem::size_of::<u32>();

    /// Creates an empty `PackedU8Vec`.
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    /// Creates a `PackedU8Vec` from a fixed-size array. Returns an error if `SIZE > 4`.
    #[inline]
    pub fn from_array<const SIZE: usize>(
        array: &[u8; SIZE],
    ) -> Result<Self, ArrayVecConstructionError> {
        if SIZE > Self::MAX_LEN {
            return Err(ArrayVecConstructionError(
                "PackedU8Vec can only hold up to 4 bytes".to_string(),
            ));
        }
        let mut arr = [0u8; Self::MAX_LEN];
        arr[..SIZE].copy_from_slice(array);
        #[cfg(target_endian = "big")]
        return Ok(Self(u32::from_le_bytes(arr)));
        #[cfg(target_endian = "little")]
        return Ok(Self(
            u32::from_be_bytes(arr) >> (8 * (Self::MAX_LEN - SIZE)),
        ));
    }

    /// Creates a `PackedU8Vec` from a byte slice. Returns an error if the slice exceeds 4 bytes.
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Result<Self, ArrayVecConstructionError> {
        if slice.len() > Self::MAX_LEN {
            return Err(ArrayVecConstructionError(
                "PackedU8Vec can only hold up to 4 bytes".to_string(),
            ));
        }
        let mut arr = [0; Self::MAX_LEN];
        arr[0..slice.len()].copy_from_slice(slice);
        #[cfg(target_endian = "big")]
        return Ok(Self(u32::from_le_bytes(arr)));
        #[cfg(target_endian = "little")]
        return Ok(Self(
            u32::from_be_bytes(arr) >> (8 * (Self::MAX_LEN - slice.len())),
        ));
    }

    /// Appends a byte. Panics if already at maximum capacity (4 bytes).
    #[inline]
    pub fn push(&mut self, value: u8) {
        if self.0.leading_zeros() < 8 {
            panic!(
                "PackedU8Vec overflow: tried to insert more than {} elements",
                Self::MAX_LEN
            );
        }
        self.0 = (self.0 << 8) | (value as u32);
    }

    /// Returns the number of stored bytes.
    #[inline]
    pub fn len(&self) -> usize {
        Self::MAX_LEN - (self.0.leading_zeros() / 8) as usize
    }

    /// Returns `true` if no bytes are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns the byte at `index`, or `None` if out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<u8> {
        let l = self.len();
        if index >= l {
            return None;
        }
        Some((self.0 >> (8 * (l - index - 1))) as u8)
    }

    /// Writes the stored bytes into the given writer. Returns the number of bytes written.
    #[inline]
    pub fn write_into(&self, mut w: impl std::io::Write) -> std::io::Result<usize> {
        let ll = self.0.leading_zeros() / 8;
        #[cfg(target_endian = "little")]
        return w.write(&u32::to_be_bytes(self.0 << (ll * 8))[..Self::MAX_LEN - ll as usize]);
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for PackedU8Vec {
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        Self::from_array(value)
    }
}

impl<const N: usize> TryFrom<[u8; N]> for PackedU8Vec {
    type Error = ArrayVecConstructionError;

    #[inline]
    fn try_from(value: [u8; N]) -> Result<Self, Self::Error> {
        Self::from_array(&value)
    }
}

#[cfg(test)]
mod tests {
    use crate::array_vec::invalid_value::ZeroValueAsInvalid;

    #[test]
    fn test_arrayvec_len() {
        use super::ArrayVec;
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = ArrayVec::new();
        assert_eq!(v.len(), 0);
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = [1u32, 2, 3].iter().cloned().collect();
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn test_arrayvec_default() {
        use super::ArrayVec;
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = ArrayVec::default();
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn test_arrayvec_from_iter() {
        use super::ArrayVec;
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = vec![1, 2, 3].into_iter().collect();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        assert_eq!(v[1], 2);
        assert_eq!(v[2], 3);
    }

    #[test]
    fn test_arrayvec_into_iter() {
        use super::ArrayVec;
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = vec![1, 2, 3].into_iter().collect();
        let mut iter = v.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_arrayvec_into_ptr_iter() {
        use super::ArrayVec;
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = vec![1, 2, 3].into_iter().collect();
        let mut iter = (&v).into_iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_arrayvec_serialization() {
        use super::ArrayVec;
        use serde_json::to_string;

        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = vec![1, 2, 3].into_iter().collect();
        let json = to_string(&v).unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[test]
    fn test_arrayvec_deserialization() {
        use super::ArrayVec;
        use serde_json::from_str;

        let json = "[1,2,3]";
        let v: ArrayVec<u32, 4, ZeroValueAsInvalid<u32>> = from_str(json).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        assert_eq!(v[1], 2);
        assert_eq!(v[2], 3);
    }

    #[test]
    fn test_arrayvecwithlen_len() {
        use super::ArrayVecWithLen;
        let v: ArrayVecWithLen<u32, 4> = ArrayVecWithLen::new();
        assert_eq!(v.len(), 0);
        let v: ArrayVecWithLen<u32, 4> = [1u32, 2, 3].iter().cloned().collect();
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn test_arrayvecwithlen_default() {
        use super::ArrayVecWithLen;
        let v: ArrayVecWithLen<u32, 4> = ArrayVecWithLen::default();
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn test_arrayvecwithlen_from_iter() {
        use super::ArrayVecWithLen;
        let v: ArrayVecWithLen<u32, 4> = vec![1, 2, 3].into_iter().collect();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        assert_eq!(v[1], 2);
        assert_eq!(v[2], 3);
    }

    #[test]
    fn test_arrayvecwithlen_into_iter() {
        use super::ArrayVecWithLen;
        let v: ArrayVecWithLen<u32, 4> = vec![1, 2, 3].into_iter().collect();
        let mut iter = v.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_arrayvecwithlen_into_ptr_iter() {
        use super::ArrayVecWithLen;
        let v: ArrayVecWithLen<u32, 4> = vec![1, 2, 3].into_iter().collect();
        let mut iter = (&v).into_iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_arrayvecwithlen_serialization() {
        use super::ArrayVecWithLen;
        use serde_json::to_string;

        let v: ArrayVecWithLen<u32, 4> = vec![1, 2, 3].into_iter().collect();
        let json = to_string(&v).unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[test]
    fn test_arrayvecwithlen_deserialization() {
        use super::ArrayVecWithLen;
        use serde_json::from_str;

        let json = "[1,2,3]";
        let v: ArrayVecWithLen<u32, 4> = from_str(json).unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        assert_eq!(v[1], 2);
        assert_eq!(v[2], 3);
    }

    #[test]
    fn test_packed_u8_vec() {
        use super::PackedU8Vec;
        let mut packed = PackedU8Vec::new();
        packed.push(1);
        packed.push(2);
        packed.push(3);
        assert_eq!(packed.len(), 3);
        assert_eq!(packed.get(0), Some(1));
        assert_eq!(packed.get(1), Some(2));
        assert_eq!(packed.get(2), Some(3));
        assert_eq!(packed.get(3), None);
        let mut buffer = Vec::new();
        packed.write_into(&mut buffer).unwrap();
        assert_eq!(
            buffer.len(),
            PackedU8Vec::MAX_LEN - packed.0.leading_zeros() as usize / 8
        );
        assert_eq!(buffer[0], 1);
        assert_eq!(buffer[1], 2);
        assert_eq!(buffer[2], 3);

        let packed = PackedU8Vec::from_array(&[1, 2, 3]).unwrap();
        assert_eq!(packed.len(), 3);
    }
}
