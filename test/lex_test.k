cond true

a ?(cond) 42 : 69
b ?(cond) 42 : 69
c ?(cond) 42 : 69
d ?(cond) 42
e ?(cond) 42

Vector3 (
  x F32
  y F32
  z F32

  cached_len F32
)

new(x F32, y F32, z F32) -> Vector3(x, y, z, 100.0)

x 10
y 20
add (x I32) -> _
func (x I32) -> I32
call () -> () -> () -> _
pwd _
f (x I32) -> _
g (x I32, y I32) -> _
T (x I32, y I32)

main() {
  v1 = ()
  v2 = (x)
  v3 = (x + y)
  v4 = (42)
  v5 = (y)
  v6 = (add(x))
  v7 = (pwd)
  v8 = (f (42))
  v9 = (g (42, 42))
  v10 = ((42, 42))
  v11 = (T(42, 42))
  v12 = (func(42) + 1)

  u = 10
  u .= 10
  u := 10
}

test() {
  call()
  call()()
  call()()()
  call()[0]
  call()[1]
  call()[x](42)
  arr1 = [1, 2, 3]
  arr2 = []
  arr3 = []42
}
