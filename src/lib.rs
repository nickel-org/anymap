//! This crate provides the `AnyMap` type, a safe and convenient store for one value of each type.

#![crate_name = "anymap"]
#![crate_type = "lib"]
#![feature(default_type_params)]
#![warn(unused_qualifications, non_upper_case_globals,
        variant_size_differences, unused_typecasts,
        missing_docs, unused_results)]

#[cfg(test)]
extern crate test;

use std::any::Any;
use std::intrinsics::{forget, TypeId};
use std::collections::HashMap;
use std::hash::{Hash, Hasher, Writer};
use std::mem::{transmute, transmute_copy};
use std::raw::TraitObject;

struct TypeIdHasher;

struct TypeIdState {
    value: u64,
}

impl Writer for TypeIdState {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        // This expects to receive one and exactly one 64-bit value
        debug_assert!(bytes.len() == 8);
        unsafe {
            std::ptr::copy_nonoverlapping_memory(&mut self.value,
                                                 transmute(&bytes[0]),
                                                 1)
        }
    }
}

impl Hasher<TypeIdState> for TypeIdHasher {
    fn hash<Sized? T: Hash<TypeIdState>>(&self, value: &T) -> u64 {
        let mut state = TypeIdState {
            value: 0,
        };
        value.hash(&mut state);
        state.value
    }
}

/// An extension of `AnyRefExt` allowing unchecked downcasting of trait objects to `&T`.
trait UncheckedAnyRefExt<'a> {
    /// Returns a reference to the boxed value, assuming that it is of type `T`. This should only be
    /// called if you are ABSOLUTELY CERTAIN of `T` as you will get really wacky output if it’s not.
    unsafe fn downcast_ref_unchecked<T: 'static>(self) -> &'a T;
}

impl<'a> UncheckedAnyRefExt<'a> for &'a (Any + 'a) {
    #[inline]
    unsafe fn downcast_ref_unchecked<T: 'static>(self) -> &'a T {
        // Get the raw representation of the trait object
        let to: TraitObject = transmute_copy(&self);

        // Extract the data pointer
        transmute(to.data)
    }
}

/// An extension of `AnyMutRefExt` allowing unchecked downcasting of trait objects to `&mut T`.
trait UncheckedAnyMutRefExt<'a> {
    /// Returns a reference to the boxed value, assuming that it is of type `T`. This should only be
    /// called if you are ABSOLUTELY CERTAIN of `T` as you will get really wacky output if it’s not.
    unsafe fn downcast_mut_unchecked<T: 'static>(self) -> &'a mut T;
}

impl<'a> UncheckedAnyMutRefExt<'a> for &'a mut (Any + 'a) {
    #[inline]
    unsafe fn downcast_mut_unchecked<T: 'static>(self) -> &'a mut T {
        // Get the raw representation of the trait object
        let to: TraitObject = transmute_copy(&self);

        // Extract the data pointer
        transmute(to.data)
    }
}

/// An extension of `BoxAny` allowing unchecked downcasting of trait objects to `Box<T>`.
trait UncheckedBoxAny {
    /// Returns the boxed value, assuming that it is of type `T`. This should only be called if you
    /// are ABSOLUTELY CERTAIN of `T` as you will get really wacky output if it’s not.
    unsafe fn downcast_unchecked<T: 'static>(self) -> Box<T>;
}

impl UncheckedBoxAny for Box<Any + 'static> {
    #[inline]
    unsafe fn downcast_unchecked<T: 'static>(self) -> Box<T> {
        // Get the raw representation of the trait object
        let to: TraitObject = *transmute::<&Box<Any>, &TraitObject>(&self);

        // Prevent destructor on self being run
        forget(self);

        // Extract the data pointer
        transmute(to.data)
    }
}

/// A map containing zero or one values for any given type and allowing convenient,
/// type-safe access to those values.
///
/// ```rust
/// # use anymap::AnyMap;
/// let mut data = AnyMap::new();
/// assert_eq!(data.get(), None::<&int>);
/// data.insert(42i);
/// assert_eq!(data.get(), Some(&42i));
/// data.remove::<int>();
/// assert_eq!(data.get::<int>(), None);
///
/// #[deriving(PartialEq, Show)]
/// struct Foo {
///     str: String,
/// }
///
/// assert_eq!(data.get::<Foo>(), None);
/// data.insert(Foo { str: "foo".to_string() });
/// assert_eq!(data.get(), Some(&Foo { str: "foo".to_string() }));
/// data.get_mut::<Foo>().map(|foo| foo.str.push('t'));
/// assert_eq!(data.get::<Foo>().unwrap().str.as_slice(), "foot");
/// ```
///
/// Values containing non-static references are not permitted.
pub struct AnyMap {
    data: HashMap<TypeId, Box<Any + 'static>, TypeIdHasher>,
}

impl AnyMap {
    /// Construct a new `AnyMap`.
    pub fn new() -> AnyMap {
        AnyMap {
            data: HashMap::with_hasher(TypeIdHasher),
        }
    }
}

impl AnyMap {
    /// Deprecated: Renamed to `get`.
    #[deprecated = "Renamed to `get`"]
    pub fn find<T: Any + 'static>(&self) -> Option<&T> {
        self.get::<T>()
    }

    /// Deprecated: Renamed to `get_mut`.
    #[deprecated = "Renamed to `get_mut`"]
    pub fn find_mut<T: Any + 'static>(&mut self) -> Option<&mut T> {
        self.get_mut::<T>()
    }

    /// Retrieve the value stored in the map for the type `T`, if it exists.
    pub fn get<T: Any + 'static>(&self) -> Option<&T> {
        self.data.get(&TypeId::of::<T>())
            .map(|any| unsafe { any.downcast_ref_unchecked::<T>() })
    }

    /// Retrieve a mutable reference to the value stored in the map for the type `T`, if it exists.
    pub fn get_mut<T: Any + 'static>(&mut self) -> Option<&mut T> {
        self.data.get_mut(&TypeId::of::<T>())
            .map(|any| unsafe { any.downcast_mut_unchecked::<T>() })
    }

    /// Set the value contained in the map for the type `T`.
    /// If there is a previous value stored, it will be returned.
    pub fn insert<T: Any + 'static>(&mut self, value: T) -> Option<T> {
        self.data.insert(TypeId::of::<T>(), box value as Box<Any>)
            .map(|any| *unsafe { any.downcast_unchecked::<T>() })
    }

    /// Remove and return the value for the type `T` if it existed.
    pub fn remove<T: Any + 'static>(&mut self) -> Option<T> {
        self.data.remove(&TypeId::of::<T>())
            .map(|any| *unsafe { any.downcast_unchecked::<T>() })
    }

    /// Does a value of type `T` exist?
    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Returns the number of items in the collection.
    pub fn len(&self) -> uint {
        self.data.len()
    }

    /// Returns true if there are no items in the collection.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Removes all items from the collection.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

#[bench]
fn bench_insertion(b: &mut ::test::Bencher) {
    b.iter(|| {
        let mut data = AnyMap::new();
        for _ in range(0u, 100) {
            let _ = data.insert(42i);
        }
    })
}

#[bench]
fn bench_get_missing(b: &mut ::test::Bencher) {
    b.iter(|| {
        let data = AnyMap::new();
        for _ in range(0u, 100) {
            assert_eq!(data.get(), None::<&int>);
        }
    })
}

#[bench]
fn bench_get_present(b: &mut ::test::Bencher) {
    b.iter(|| {
        let mut data = AnyMap::new();
        let _ = data.insert(42i);
        // These inner loops are a feeble attempt to drown the other factors.
        for _ in range(0u, 100) {
            assert_eq!(data.get(), Some(&42i));
        }
    })
}
