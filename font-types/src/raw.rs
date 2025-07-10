//! types for working with raw big-endian and little-endian bytes

/// A trait for font scalars.
///
/// This is an internal trait for encoding and decoding raw bytes.
///
/// You do not need to implement this trait directly; it is an implementation
/// detail of the [`BigEndian`]/[`LittleEndian`] wrapper.
pub trait Scalar: Sized {
    /// The raw byte representation of this type.
    type Raw: sealed::BackingByteArray;

    /// Create an instance of this type from raw big-endian bytes
    fn from_raw_be(raw: Self::Raw) -> Self;

    /// Encode this type as raw big-endian bytes
    fn to_raw_be(self) -> Self::Raw;

    /// Attempt to read a scalar from a big-endian slice.
    ///
    /// This will always succeed if `slice.len() == Self::RAW_BYTE_LEN`, and will
    /// always return `None` otherwise.
    fn read_be(slice: &[u8]) -> Option<Self> {
        sealed::BackingByteArray::from_slice(slice).map(Self::from_raw_be)
    }

    /// Create an instance of this type from raw little-endian bytes
    fn from_raw_le(raw: Self::Raw) -> Self;

    /// Encode this type as raw little-endian bytes
    fn to_raw_le(self) -> Self::Raw;

    /// Attempt to read a scalar from a little-endian slice.
    ///
    /// This will always succeed if `slice.len() == Self::RAW_BYTE_LEN`, and will
    /// always return `None` otherwise.
    fn read_le(slice: &[u8]) -> Option<Self> {
        sealed::BackingByteArray::from_slice(slice).map(Self::from_raw_le)
    }
}

/// Convenience trait for newtypes which wrap a [`Scalar`]. The [`Scalar`] trait
/// is automatically implemented for all types that implement this trait.
pub trait ScalarWrapper {
    #[doc(hidden)]
    type Inner: Scalar;

    #[doc(hidden)]
    fn from_inner(inner: Self::Inner) -> Self;
    #[doc(hidden)]
    fn into_inner(self) -> Self::Inner;
}

impl<T: ScalarWrapper> Scalar for T {
    type Raw = <T::Inner as Scalar>::Raw;

    #[inline(always)]
    fn from_raw_be(raw: Self::Raw) -> Self {
        Self::from_inner(T::Inner::from_raw_be(raw))
    }

    #[inline(always)]
    fn to_raw_be(self) -> Self::Raw {
        self.into_inner().to_raw_be()
    }

    #[inline(always)]
    fn from_raw_le(raw: Self::Raw) -> Self {
        Self::from_inner(T::Inner::from_raw_le(raw))
    }

    #[inline(always)]
    fn to_raw_le(self) -> Self::Raw {
        self.into_inner().to_raw_le()
    }
}

/// A trait for types that have a known, constant size.
pub trait FixedSize: Sized {
    /// The raw size of this type, in bytes.
    ///
    /// This is the size required to represent this type in a font file, which
    /// may differ from the size of the native type:
    ///
    /// ```
    /// # use font_types::{FixedSize, Offset24};
    /// assert_eq!(std::mem::size_of::<u16>(), u16::RAW_BYTE_LEN);
    /// assert_eq!(Offset24::RAW_BYTE_LEN, 3);
    /// assert_eq!(std::mem::size_of::<Offset24>(), 4);
    /// ```
    const RAW_BYTE_LEN: usize;
}

/// we hide this trait; it isn't part of the public API, and this clarifies
/// the guarantee that it is only implemented for [u8; N]
pub(super) mod sealed {
    /// A trait representing any fixed-size backing byte array.
    ///
    /// This is only used in `Scalar`, as a way of expressing the condition that the
    /// `Raw` type is always a fixed-size byte array.
    #[cfg(not(feature = "bytemuck"))]
    pub trait BackingByteArray: Copy + AsRef<[u8]> {
        /// Must always succeed for `[u8; N]` if `slice.len() == N`, must fail otherwise
        fn from_slice(slice: &[u8]) -> Option<Self>;
    }
    #[cfg(feature = "bytemuck")]
    pub trait BackingByteArray:
        Copy + AsRef<[u8]> + bytemuck::AnyBitPattern + bytemuck::Zeroable
    {
        /// Must always succeed for `[u8; N]` if `slice.len() == N`, must fail otherwise
        fn from_slice(slice: &[u8]) -> Option<Self>;
    }

    impl<const N: usize> BackingByteArray for [u8; N] {
        fn from_slice(slice: &[u8]) -> Option<Self> {
            slice.try_into().ok()
        }
    }
}

/// A trait for types that contain a byte slice which can be decoded to a scalar value.
pub trait BytesWrapper: Sized {
    type Inner: Sized;
    /// Attempt to construct a new raw value from this slice.
    ///
    /// This will fail if `slice.len() != T::RAW_BYTE_LEN`.
    fn from_slice(slice: &[u8]) -> Option<Self>;

    /// Get the scalar value of the bytes contained in this byte wrapper.
    fn get(&self) -> Self::Inner;

    /// Set the value, overwriting the bytes.
    fn set(&mut self, value: Self::Inner);
}

macro_rules! impl_traits_for_wrapper {
    ($wrapper:ident) => {
        // # SAFETY:
        //
        // `BigEndian<T>`/`LittleEndian<T>` has the bound `T: Scalar`, and
        // contains only a single value, `<T as Scalar>::Raw` which is only ever
        // a byte array.
        #[cfg(feature = "bytemuck")]
        unsafe impl<T> bytemuck::Zeroable for $wrapper<T> where T: Scalar + Copy {}
        #[cfg(feature = "bytemuck")]
        unsafe impl<T> bytemuck::AnyBitPattern for $wrapper<T> where T: Scalar + Copy + 'static {}

        impl<T: Scalar + Default> Default for $wrapper<T> {
            fn default() -> Self {
                Self::from(T::default())
            }
        }

        impl<T: Scalar> FixedSize for $wrapper<T> {
            const RAW_BYTE_LEN: usize = T::RAW_BYTE_LEN;
        }

        //NOTE: due to the orphan rules, we cannot impl the inverse of this,
        // e.g. impl<T> PartialEq<BigEndian<T>> for T
        // (<https://doc.rust-lang.org/error_codes/E0210.html>)
        impl<T: Scalar + Copy + PartialEq> PartialEq<T> for $wrapper<T> {
            fn eq(&self, other: &T) -> bool {
                self.get() == *other
            }
        }

        impl<T: Scalar + Copy + PartialOrd + PartialEq> PartialOrd for $wrapper<T>
        where
            <T as Scalar>::Raw: PartialEq,
        {
            fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
                self.get().partial_cmp(&other.get())
            }
        }

        impl<T: Scalar + Copy + Ord + Eq> Ord for $wrapper<T>
        where
            <T as Scalar>::Raw: Eq,
        {
            fn cmp(&self, other: &Self) -> core::cmp::Ordering {
                self.get().cmp(&other.get())
            }
        }

        impl<T: std::fmt::Debug + Scalar + Copy> std::fmt::Debug for $wrapper<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.get().fmt(f)
            }
        }

        impl<T: std::fmt::Display + Scalar + Copy> std::fmt::Display for $wrapper<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.get().fmt(f)
            }
        }
    };
}

/// A wrapper around raw big-endian bytes for some type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct BigEndian<T: Scalar>(pub(crate) T::Raw);

impl<T: Scalar> BytesWrapper for BigEndian<T> {
    type Inner = T;
    fn from_slice(slice: &[u8]) -> Option<Self> {
        sealed::BackingByteArray::from_slice(slice).map(Self)
    }

    #[inline(always)]
    fn get(&self) -> T {
        T::from_raw_be(self.0)
    }

    fn set(&mut self, value: T) {
        self.0 = value.to_raw_be();
    }
}

impl<T: Scalar> BigEndian<T> {
    /// construct a new `BigEndian<T>` from raw bytes
    pub fn new(raw: T::Raw) -> BigEndian<T> {
        BigEndian(raw)
    }

    /// Get the raw big-endian bytes.
    pub fn be_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T: Scalar> From<T> for BigEndian<T> {
    #[inline]
    fn from(val: T) -> Self {
        BigEndian(val.to_raw_be())
    }
}

impl_traits_for_wrapper!(BigEndian);

/// A wrapper around raw big-endian bytes for some type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct LittleEndian<T: Scalar>(pub(crate) T::Raw);

impl<T: Scalar> BytesWrapper for LittleEndian<T> {
    type Inner = T;
    fn from_slice(slice: &[u8]) -> Option<Self> {
        sealed::BackingByteArray::from_slice(slice).map(Self)
    }

    #[inline(always)]
    fn get(&self) -> T {
        T::from_raw_le(self.0)
    }

    fn set(&mut self, value: T) {
        self.0 = value.to_raw_le();
    }
}

impl<T: Scalar> LittleEndian<T> {
    /// construct a new `LittleEndian<T>` from raw bytes
    pub fn new(raw: T::Raw) -> LittleEndian<T> {
        LittleEndian(raw)
    }

    /// Get the raw little-endian bytes.
    pub fn le_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T: Scalar> From<T> for LittleEndian<T> {
    #[inline]
    fn from(val: T) -> Self {
        LittleEndian(val.to_raw_le())
    }
}

impl_traits_for_wrapper!(LittleEndian);

impl<T: Scalar> FixedSize for T {
    const RAW_BYTE_LEN: usize = std::mem::size_of::<T::Raw>();
}

/// An internal macro for implementing the `RawType` trait.
#[macro_export]
macro_rules! newtype_scalar {
    ($ty:ident, $raw:ty) => {
        impl $crate::raw::Scalar for $ty {
            type Raw = $raw;
            #[inline(always)]
            fn to_raw_be(self) -> $raw {
                self.0.to_raw_be()
            }

            #[inline(always)]
            fn from_raw_be(raw: $raw) -> Self {
                Self($crate::raw::Scalar::from_raw_be(raw))
            }

            #[inline(always)]
            fn to_raw_le(self) -> $raw {
                self.0.to_raw_le()
            }

            #[inline(always)]
            fn from_raw_le(raw: $raw) -> Self {
                Self($crate::raw::Scalar::from_raw_le(raw))
            }
        }
    };
}

macro_rules! int_scalar {
    ($ty:ty, $raw:ty) => {
        impl crate::raw::Scalar for $ty {
            type Raw = $raw;
            #[inline(always)]
            fn to_raw_be(self) -> $raw {
                self.to_be_bytes()
            }

            #[inline(always)]
            fn from_raw_be(raw: $raw) -> $ty {
                Self::from_be_bytes(raw)
            }

            #[inline(always)]
            fn to_raw_le(self) -> $raw {
                self.to_le_bytes()
            }

            #[inline(always)]
            fn from_raw_le(raw: $raw) -> $ty {
                Self::from_le_bytes(raw)
            }
        }
    };
}

int_scalar!(u8, [u8; 1]);
int_scalar!(i8, [u8; 1]);
int_scalar!(u16, [u8; 2]);
int_scalar!(i16, [u8; 2]);
int_scalar!(u32, [u8; 4]);
int_scalar!(i32, [u8; 4]);
int_scalar!(i64, [u8; 8]);
int_scalar!(crate::Uint24, [u8; 3]);
int_scalar!(crate::Int24, [u8; 3]);
