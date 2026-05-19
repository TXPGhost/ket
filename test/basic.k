print (msg String) -> _

main() {
  print("hello, world")
}

add(x I32, y 0) -> x
add2(x I32, y I32) -> 1 + add2(x, y - 1)
