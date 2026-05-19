Particle _
Rocket _
Emitter _

World (
  particles []Particle
  rockets []Rocket
  emitters []Emitter

  sock Socket
)

init_world() {
  World([], [], [], Socket("127.0.0.1"))
}

add_particle(world World, particle Particle) {}
add_rocket(world World, rocket Rocket) {}
add_emitter(world World, emitter Emitter) {}

Socket (addr String)

print(sock Socket) -> _

main() {
  my_world = init_world()

  my_world:add_particle(Particle)
  my_world:add_rocket(Rocket)
  my_world:add_emitter(Emitter)

  x = y
  y = 10
}
