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
  my_sock = Socket(x="127.0.0.1:8080")

  my_sock:close()

  x = []
}
