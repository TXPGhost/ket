Recursive (x F32, y F32, z F32, w Recursive)

generic_vector3 (
  float (
    Vector3 (
      x F32
      y F32
      z F32
    )
  )

  int (
    Vector3 (
      x I32
      y I32
      z I32
    )
  )
)

x generic_vector3

Hello (
  world (x 10, y 20)
  abc Hello
)

main() {
  my_vec = generic_vector3.float.Vector3.x + generic_vector3.float.Vector3.y

  test = generic_vector3.float.Vector3(x=1.0, y=2.0, z=3.0)

  my_tuple = ((("hello, world!", 42 * 100 + 10)))
  x = my_tuple.1

  test = my_tuple.0

  hello = Hello

  uoasboub = 100

  arr = [1, 1, 5]

  main() + arr
}

true 1
false 0

opts (
  keymap (preset "none")
  appearance (nerd_font_variant "normal")
  cmdline (
    enabled true
    keymap (preset "inherit")
    completion (
      menu (auto_show "true")
      list (selection (preselect false, auto_insert false))
    )
  )
)
