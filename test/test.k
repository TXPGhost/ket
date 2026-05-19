Test (x I32)

TestFn (x I32) -> x

Vector(T _) (
  buf []T
)

func () -> () -> 10

Arena (
  num_allocs I32
)

new(Ty Arena) -> Ty

assert_eq(lhs I32, rhs I32) -> Bool

test_arena_new() {
  arena = Arena:new()
  assert_eq(0, arena.num_allocs)
}
