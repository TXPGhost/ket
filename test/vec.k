Vector3 (
  x F32
  y F32
  z F32
)

new(x F32, y F32, z F32) Vector3(x, y, z)

add(lhs Vector3, rhs Vector3) Vector3(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
sub(lhs Vector3, rhs Vector3) Vector3(lhs.x - rhs.x, lhs.y - rhs.y, lhs.z - rhs.z)

to_string(val Vector3) String

print(msg String) _

main() {
  vec1 = Vector3(1.0, 2.0, 2.0)
  vec2 = Vector3(2.0, 0.0, 3.0)

  vec3 = add(vec1, vec2)

  print(to_string(vec3))
}
