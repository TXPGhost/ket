a ?(cond) 42 : 69
b ?(cond) 42 : 69
c ?(cond) 42 : 69
d ?(cond) 42
e ?(cond) 42
f ?(cond) 42 : 69

Vector3 (
  x F32
  y F32
  z F32

  cached (len F32)

  new(x F32, y F32, z F32) Vector3(x, y, z)
)

main() {
  v = ()
  v = (x)
  v = (x + y)
  v = (42)
  v = (y)
  v = (add(x))
  v = (pwd)
  v = (x (42))
  v = (x (42, 42))
  v = ((42, 42))
  v = (T(42, 42))
  v = (func(42) + 1)

  v = 10
  v .= 10
  v := 10
}

Vector3 (
  new(x F32, y F32, z F32) Vector3(x, y, z)
  elts (x F32, y F32, z F32)
)

test() {
  call()
  call()()
  call()()()
  call()[0]
  call()[1]
  call()[x](42)
  arr = [1, 2, 3]
  arr = []
  arr = []42
}

Vector3 I32?"error message"
