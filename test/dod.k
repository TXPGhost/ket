Particle _
Rocket _
Emitter _

World (
  particles []Particle
  rockets []Rocket
  emitters []Emitter?

  sock Socket
)

init_world() {

}

Socket (addr String)

main() {
  my_sock = Socket(addr="127.0.0.1:8080")

  names = ["Alexander", "James", "Robertson"]

  arr1 = [true, true, true]
  arr2 = [true, true, false]
  arr3 = [true, _, _, _]

  x = String?
  y = String

  ?(x) {
    w = 10
  }

  weird = Bool?
  z = print(my_sock)

  ?(weird) {
    x1 = weird
  } : {
    x2 = weird
  }

  z = [x, y]
}
