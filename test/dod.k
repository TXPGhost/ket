Particle _
Rocket _
Emitter _

World (
  particles []Particle
  rockets []Rocket
  emitters []Emitter?
)

init_world() {

}

Socket (addr String)

main() {
  my_sock = Socket(addr="127.0.0.1:8080")

  arr = [true, true, true]
  arr = [true, true, false]
  arr = [true, 42]

  x = []
}
