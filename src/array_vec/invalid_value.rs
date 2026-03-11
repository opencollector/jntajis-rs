use serde::{Deserialize, Serialize};

/// Trait for determining whether a value is valid or represents a sentinel "invalid" state.
///
/// Used by [`ArrayVec`](super::ArrayVec) to distinguish occupied slots from empty ones
/// without storing an explicit length.
pub trait ValueValidity {
    /// The value type being validated.
    type Target;

    /// Returns the sentinel value representing an invalid/empty slot.
    fn invalid_value() -> Self::Target;

    /// Returns `true` if the value is valid (i.e., not the sentinel).
    fn is_valid(value: &Self::Target) -> bool;
}

/// Default validity strategy that delegates to the inner type's [`ValueValidity`] implementation.
///
/// Provides built-in support for `Option<T>` (`None` is invalid), `*const T` and `*mut T`
/// (null is invalid).
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct DefaultValueValidity<T> {
    _a: std::marker::PhantomData<T>,
}

impl<T: ValueValidity> ValueValidity for DefaultValueValidity<T> {
    type Target = T::Target;

    fn invalid_value() -> Self::Target {
        T::invalid_value()
    }

    fn is_valid(value: &Self::Target) -> bool {
        T::is_valid(value)
    }
}

impl<T> ValueValidity for DefaultValueValidity<Option<T>> {
    type Target = Option<T>;

    fn invalid_value() -> Self::Target {
        None
    }

    fn is_valid(value: &Self::Target) -> bool {
        value.is_some()
    }
}

impl<T> ValueValidity for DefaultValueValidity<*const T> {
    type Target = *const T;

    fn invalid_value() -> Self::Target {
        std::ptr::null()
    }

    fn is_valid(value: &Self::Target) -> bool {
        !value.is_null()
    }
}

impl<T> ValueValidity for DefaultValueValidity<*mut T> {
    type Target = *mut T;

    fn invalid_value() -> Self::Target {
        std::ptr::null_mut()
    }

    fn is_valid(value: &Self::Target) -> bool {
        !value.is_null()
    }
}

/// Validity strategy that treats the zero (all-bits-zero) value as invalid.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[allow(dead_code)]
pub struct ZeroValueAsInvalid<T: PartialEq> {
    _a: std::marker::PhantomData<T>,
}

impl<T: PartialEq> ValueValidity for ZeroValueAsInvalid<T> {
    type Target = T;

    fn invalid_value() -> Self::Target {
        unsafe { std::mem::zeroed() }
    }

    fn is_valid(value: &Self::Target) -> bool {
        *value != Self::invalid_value()
    }
}

/// Validity strategy that treats the all-bits-set value as invalid.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct AllBitsSetValueAsInvalid<T: PartialEq + std::ops::Not<Output = T>>(
    std::marker::PhantomData<T>,
);

impl<T: PartialEq + std::ops::Not<Output = T>> ValueValidity for AllBitsSetValueAsInvalid<T> {
    type Target = T;

    fn invalid_value() -> Self::Target {
        unsafe { !std::mem::zeroed::<T>() }
    }

    fn is_valid(value: &Self::Target) -> bool {
        *value != Self::invalid_value()
    }
}

/// Validity strategy that treats the [`Default`] value as invalid.
#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[allow(dead_code)]
pub struct DefaultValueAsInvalid<T: PartialEq + Default> {
    _a: std::marker::PhantomData<T>,
}

impl<T: PartialEq + Default> ValueValidity for DefaultValueAsInvalid<T> {
    type Target = T;

    fn invalid_value() -> Self::Target {
        T::default()
    }

    fn is_valid(value: &Self::Target) -> bool {
        *value != Self::invalid_value()
    }
}
