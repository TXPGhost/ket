Vector3 (
  x F32
  y F32
  z F32
)

new(x F32, y F32, z F32) Vector3(x, y, z)

add(lhs Vector3, rhs Vector3) Vector3(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
sub(lhs Vector3, rhs Vector3) Vector3(lhs.x - rhs.x, lhs.y - rhs.y, lhs.z - rhs.z)
