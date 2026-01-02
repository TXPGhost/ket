use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZero;

/// Indicates that a type can be used as an arena index. Requires that the type is copyable, has a
/// maximum value, and has a one-to-one mapping to values of type [usize].
pub trait Index: Copy + PartialEq + Eq {
    /// The maximum [usize] value this index can represent as a, e.g. 256 for an 8-bit unsigned integer.
    const MAX: usize;

    /// Converts this value into a value of type [usize].
    fn into_usize(self) -> usize;

    /// Constructs this value from a value of type [usize].
    fn from_usize(index: usize) -> Self;
}

impl Index for u8 {
    const MAX: usize = u8::MAX as usize;
    fn into_usize(self) -> usize {
        self as usize
    }
    fn from_usize(index: usize) -> Self {
        index as u8
    }
}
impl Index for u16 {
    const MAX: usize = u16::MAX as usize;
    fn into_usize(self) -> usize {
        self as usize
    }
    fn from_usize(index: usize) -> Self {
        index as u16
    }
}
impl Index for u32 {
    const MAX: usize = u32::MAX as usize;
    fn into_usize(self) -> usize {
        self as usize
    }
    fn from_usize(index: usize) -> Self {
        index as u32
    }
}
impl Index for u64 {
    const MAX: usize = u64::MAX as usize;
    fn into_usize(self) -> usize {
        self as usize
    }
    fn from_usize(index: usize) -> Self {
        index as u64
    }
}

impl Index for NonZero<u8> {
    const MAX: usize = u8::MAX as usize - 1;
    fn into_usize(self) -> usize {
        (self.get() - 1) as usize
    }
    fn from_usize(index: usize) -> Self {
        NonZero::new(index as u8 + 1).unwrap()
    }
}
impl Index for NonZero<u16> {
    const MAX: usize = u16::MAX as usize - 1;
    fn into_usize(self) -> usize {
        (self.get() - 1) as usize
    }
    fn from_usize(index: usize) -> Self {
        NonZero::new(index as u16 + 1).unwrap()
    }
}
impl Index for NonZero<u32> {
    const MAX: usize = u32::MAX as usize - 1;
    fn into_usize(self) -> usize {
        (self.get() - 1) as usize
    }
    fn from_usize(index: usize) -> Self {
        NonZero::new(index as u32 + 1).unwrap()
    }
}
impl Index for NonZero<u64> {
    const MAX: usize = u64::MAX as usize - 1;
    fn into_usize(self) -> usize {
        (self.get() - 1) as usize
    }
    fn from_usize(index: usize) -> Self {
        NonZero::new(index as u64 + 1).unwrap()
    }
}

/// The default arena index type. Uses 32 bytes by default since most arenas don't need more than
/// 2^32 elements, also with [NonZero] so that [Option] doesn't take up any additional space.
pub type DefaultIndex = NonZero<u32>;

/// A unique index-based identifier for a resource in an [Arena], with index type `I` and marker
/// type `M`, which prevents accidentally using this [Ref] on the wrong [Arena].
pub struct Ref<M, I: Index = DefaultIndex>(I, PhantomData<M>);

impl<M, I: Index> PartialEq for Ref<M, I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<M, I: Index> Eq for Ref<M, I> {}
impl<M, I: Index> Clone for Ref<M, I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, I: Index> Copy for Ref<M, I> {}
impl<M, I: Index> Debug for Ref<M, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", std::any::type_name::<M>(), self.0.into_usize())
    }
}

impl<M, I: Index> Ref<M, I> {
    /// Immutably borrows the value at the [Arena] referenced by this [Ref].
    pub fn get<T>(self, arena: &Arena<M, T, I>) -> &T {
        &arena.buf[self.0.into_usize()]
    }

    /// Mutably borrows the value at the [Arena] referenced by this [Ref].
    pub fn get_mut<T>(self, arena: &mut Arena<M, T, I>) -> &mut T {
        &mut arena.buf[self.0.into_usize()]
    }

    /// Puts a value into the [Arena] slot referenced by this [Ref]. Since the [Arena] needs to be
    /// populated with some data, we rely on the [Default] implementation of `T`. Calling this
    /// function potentially may create lots of default elements, so consider wrapping in an
    /// [Option] if the constructor is expensive.
    pub fn put<T: Default>(self, arena: &mut Arena<M, T, I>, val: T) {
        while arena.buf.len() < self.0.into_usize() {
            arena.alloc(T::default());
        }
        if arena.buf.len() == self.0.into_usize() {
            arena.buf.push(val);
        } else {
            arena.buf[self.0.into_usize()] = val;
        }
    }
}

/// An efficient way to allocate objects of type `T` using a [Vec] under the hood, returns unique
/// identifiers of type [Ref] using index type `I`. Also takes a marker type `M`, which prevents
/// accidentally using the wrong [Ref] for this [Arena].
pub struct Arena<M, T, I: Index = DefaultIndex> {
    buf: Vec<T>,
    _phantom: (PhantomData<I>, PhantomData<M>),
}

impl<M, T, I: Index> Arena<M, T, I> {
    /// Constructs a new [Arena], initially empty.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Returns the number of current allocations.
    pub fn num_allocations(&self) -> usize {
        self.buf.len()
    }

    /// Allocates space for a value of type `T` and inserts it into the [Arena], returning an [Ref].
    pub fn alloc(&mut self, val: T) -> Ref<M, I> {
        if self.buf.len() == I::MAX {
            panic!("out of space");
        }
        self.buf.push(val);
        Ref(I::from_usize(self.buf.len() - 1), PhantomData)
    }

    /// Returns an iterator over references to the elements within this [Arena].
    pub fn iter(&self) -> impl Iterator<Item = Ref<M, I>> {
        (0..self.buf.len()).map(|idx| Ref(I::from_usize(idx), PhantomData))
    }
}

impl<M, T, I: Index> Default for Arena<M, T, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, T: Debug, I: Index> Debug for Arena<M, T, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.buf.iter().enumerate()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker;

    #[test]
    fn test_arena_new() {
        let arena = Arena::<Marker, String>::new();
        assert_eq!(0, arena.num_allocations());
    }

    #[test]
    fn test_arena_alloc_one() {
        let mut arena = Arena::<Marker, String>::new();
        arena.alloc(String::from("Hello, world!"));
        assert_eq!(1, arena.num_allocations());
    }

    #[test]
    fn test_arena_alloc_many() {
        let mut arena = Arena::<Marker, String>::new();
        arena.alloc(String::from("first string"));
        arena.alloc(String::from("second string"));
        arena.alloc(String::from("third string"));
        assert_eq!(3, arena.num_allocations());
    }

    #[test]
    fn test_arena_id_get() {
        let mut arena = Arena::<Marker, String>::new();
        let id = arena.alloc(String::from("Hello, world!"));
        assert_eq!("Hello, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocations());
    }

    #[test]
    fn test_arena_id_get_mut() {
        let mut arena = Arena::<Marker, String>::new();
        let id = arena.alloc(String::from("Hello, world!"));
        *id.get_mut(&mut arena) = String::from("Goodbye, world!");
        assert_eq!("Goodbye, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocations());
    }

    #[test]
    fn test_arena_id_put() {
        let mut arena = Arena::<Marker, String>::new();
        let id = arena.alloc(String::from("Hello, world!"));
        id.put(&mut arena, String::from("Goodbye, world!"));
        assert_eq!("Goodbye, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocations());
    }

    #[test]
    fn test_arena_id_put_oob() {
        let mut arena1 = Arena::<Marker, String>::new();
        let mut arena2 = Arena::<Marker, String>::new();
        arena1.alloc(String::new());
        arena1.alloc(String::new());
        arena1.alloc(String::new());
        let id = arena1.alloc(String::from("Hello, world!"));
        id.put(&mut arena2, String::from("Goodbye, world!"));
        assert_eq!(4, arena1.num_allocations());
        assert_eq!(4, arena2.num_allocations());
    }
}
