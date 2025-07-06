use serde::{Deserialize, Serialize};

pub trait ValueValidity {
    type Target;

    fn invalid_value() -> Self::Target;

    fn is_valid(value: &Self::Target) -> bool;
}

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
