use std::fmt::Debug;
use std::hash::Hash;
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
/// type `M`, which prevents accidentally using this [Id] on the wrong [Arena].
pub struct Id<M, I: Index = DefaultIndex>(I, PhantomData<M>);

impl<M, I: Index> PartialEq for Id<M, I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<M, I: Index> Eq for Id<M, I> {}
impl<M, I: Index> Clone for Id<M, I> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, I: Index> Copy for Id<M, I> {}
impl<M, I: Index> Debug for Id<M, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", std::any::type_name::<M>(), self.0.into_usize())
    }
}

impl<M, I: Index> Hash for Id<M, I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index().hash(state);
    }
}

impl<M, I: Index> Id<M, I> {
    /// Immutably borrows the value at the [Arena] referenced by this [Id].
    pub fn get<T: Default>(self, arena: &Arena<M, T, I>) -> &T {
        arena.buf.get(self.0.into_usize()).unwrap_or(&arena.default)
    }

    /// Mutably borrows the value at the [Arena] referenced by this [Id].
    pub fn get_mut<T: Default>(self, arena: &mut Arena<M, T, I>) -> &mut T {
        while arena.buf.len() <= self.0.into_usize() {
            arena.buf.push(T::default());
        }
        &mut arena.buf[self.0.into_usize()]
    }

    /// Puts a value into the [Arena] slot referenced by this [Id], returning [self] for
    /// convenience. Since the [Arena] needs to be populated with some data, we rely on the
    /// [Default] implementation of `T`. Calling this function potentially may create lots of
    /// default elements, so consider wrapping `T` in an [Option] if the constructor is expensive.
    pub fn put<T: Default>(self, arena: &mut Arena<M, T, I>, val: T) -> Self {
        while arena.buf.len() < self.0.into_usize() {
            arena.buf.push(T::default());
        }
        if arena.buf.len() == self.0.into_usize() {
            arena.buf.push(val);
        } else {
            arena.buf[self.0.into_usize()] = val;
        }
        self
    }

    /// Takes a value out of the [Arena] slot referenced by this [Id], replacing it with the
    /// [Default] value.
    pub fn take<T: Default>(self, arena: &mut Arena<M, T, I>) -> T {
        while arena.buf.len() <= self.0.into_usize() {
            arena.buf.push(T::default());
        }
        std::mem::take(&mut arena.buf[self.0.into_usize()])
    }

    /// Constructs a new [Id] from an index.
    pub fn new(index: usize) -> Self {
        if index >= I::MAX {
            panic!("index exceeds maximum");
        }
        Id(I::from_usize(index), PhantomData)
    }

    /// Constructs the index associated with this [Id].
    pub fn index(self) -> usize {
        I::into_usize(self.0)
    }

    /// Returns the next adjacent [Id] to this one.
    pub fn next(self) -> Option<Self> {
        let index = self.0.into_usize().checked_add(1)?;
        if index >= I::MAX {
            return None;
        }
        Some(Id(I::from_usize(index), PhantomData))
    }

    /// Returns the previous adjacent [Id] to this one.
    pub fn prev(self) -> Option<Self> {
        let index = self.0.into_usize().checked_sub(1)?;
        Some(Id(I::from_usize(index), PhantomData))
    }
}

#[derive(Debug)]
pub struct World<M, I: Index = DefaultIndex> {
    next_index: usize,
    has_space: bool,
    _phantom: (PhantomData<M>, PhantomData<I>),
}
impl<M, I: Index> World<M, I> {
    pub fn new() -> Self {
        Self {
            next_index: 0,
            has_space: I::MAX != 0,
            _phantom: (PhantomData, PhantomData),
        }
    }

    pub fn alloc(&mut self) -> Id<M, I> {
        if !self.has_space {
            panic!("out of space");
        }
        let index = self.next_index;
        self.next_index += 1;
        self.has_space = index != I::MAX;
        Id(I::from_usize(index), PhantomData)
    }

    /// Returns an iterator over ids within this [World].
    pub fn iter(&self) -> impl Iterator<Item = Id<M, I>> {
        (0..self.next_index).map(|idx| Id(I::from_usize(idx), PhantomData))
    }

    /// Returns the number of provided ids.
    pub fn num_allocs(&self) -> usize {
        self.next_index
    }
}

impl<M, I: Index> Default for World<M, I> {
    fn default() -> Self {
        Self::new()
    }
}

/// An efficient way to allocate objects of type `T` using a [Vec] under the hood, returns unique
/// identifiers of type [Id] using index type `I`. Also takes a marker type `M`, which prevents
/// accidentally using the wrong [Id] for this [Arena].
pub struct Arena<M, T: Default, I: Index = DefaultIndex> {
    buf: Vec<T>,
    default: T,
    _phantom: (PhantomData<I>, PhantomData<M>),
}

impl<M, T: Default, I: Index> Arena<M, T, I> {
    /// Constructs a new [Arena], initially empty.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            default: T::default(),
            _phantom: (PhantomData, PhantomData),
        }
    }

    /// Returns the number of current allocations.
    pub fn num_allocs(&self) -> usize {
        self.buf.len()
    }

    /// Returns an iterator over ids within this [Arena].
    pub fn iter(&self) -> impl Iterator<Item = Id<M, I>> {
        (0..self.buf.len()).map(|idx| Id(I::from_usize(idx), PhantomData))
    }
}

impl<M, T: Default, I: Index> Default for Arena<M, T, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M, T: Default + Debug, I: Index> Debug for Arena<M, T, I> {
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
        assert_eq!(0, arena.num_allocs());
    }

    #[test]
    fn test_arena_alloc_one() {
        let mut world = World::<Marker>::new();
        let mut arena = Arena::<Marker, String>::new();
        world.alloc().put(&mut arena, String::from("Hello, world!"));
        assert_eq!(1, arena.num_allocs());
    }

    #[test]
    fn test_arena_alloc_many() {
        let mut world = World::<Marker>::new();
        let mut arena = Arena::<Marker, String>::new();
        world.alloc().put(&mut arena, String::from("first string"));
        world.alloc().put(&mut arena, String::from("second string"));
        world.alloc().put(&mut arena, String::from("third string"));
        assert_eq!(3, arena.num_allocs());
    }

    #[test]
    fn test_arena_id_get() {
        let mut world = World::<Marker>::new();
        let mut arena = Arena::<Marker, String>::new();
        let id = world.alloc().put(&mut arena, String::from("Hello, world!"));
        assert_eq!("Hello, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocs());
    }

    #[test]
    fn test_arena_id_get_mut() {
        let mut world = World::<Marker>::new();
        let mut arena = Arena::<Marker, String>::new();
        let id = world.alloc().put(&mut arena, String::from("Hello, world!"));
        *id.get_mut(&mut arena) = String::from("Goodbye, world!");
        assert_eq!("Goodbye, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocs());
    }

    #[test]
    fn test_arena_id_put() {
        let mut world = World::<Marker>::new();
        let mut arena = Arena::<Marker, String>::new();
        let id = world.alloc().put(&mut arena, String::from("Hello, world!"));
        id.put(&mut arena, String::from("Goodbye, world!"));
        assert_eq!("Goodbye, world!", id.get(&arena).as_str());
        assert_eq!(1, arena.num_allocs());
    }

    #[test]
    fn test_arena_id_put_oob() {
        let mut world = World::<Marker>::new();
        let mut arena1 = Arena::<Marker, String>::new();
        let mut arena2 = Arena::<Marker, String>::new();
        world.alloc().put(&mut arena1, String::new());
        world.alloc().put(&mut arena1, String::new());
        world.alloc().put(&mut arena1, String::new());
        let id = world
            .alloc()
            .put(&mut arena1, String::from("Hello, world!"));
        id.put(&mut arena2, String::from("Goodbye, world!"));
        assert_eq!(4, arena1.num_allocs());
        assert_eq!(4, arena2.num_allocs());
    }
}
