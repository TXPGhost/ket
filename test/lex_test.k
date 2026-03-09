cond true

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

  cached_len F32
)

new(x F32, y F32, z F32) Vector3(x, y, z, 100.0)

x 10
y 20
add (x I32) _
func (x I32) I32
call () _
pwd _
f (x I32) _
g (x I32, y I32) _
T (x I32, y I32)

main() {
  v = ()
  v = (x)
  v = (x + y)
  v = (42)
  v = (y)
  v = (add(x))
  v = (pwd)
  v = (f (42))
  v = (g (42, 42))
  v = ((42, 42))
  v = (T(42, 42))
  v = (func(42) + 1)

  v = 10
  v .= 10
  v := 10
}

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
