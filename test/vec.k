Vector3 (x F32, y F32, z F32)

zero Vector3(0.0, 0.0, 0.0)

unit_x Vector3(1.0, 0.0, 0.0)
unit_y Vector3(0.0, 1.0, 0.0)
unit_z Vector3(0.0, 0.0, 1.0)

vector3(x F32, y F32, z F32) Vector3(x, y, z)

add(lhs Vector3, rhs Vector3) {
  Vector3(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
}
sub(lhs Vector3, rhs Vector3) {
  add(lhs, neg(rhs))
}
neg(self Vector3) Vector3(0.0-self.x, 0.0-self.y, 0.0-self.z)
mul(lhs Vector3, rhs F32) {
  Vector3(lhs.x * rhs, lhs.y * rhs, lhs.z * rhs)
}

main() {
  lhs = unit_x
  rhs = unit_y

  sum = add(lhs, rhss)
}
