Vector3 (x F32, y F32, z F32)

recursive() { recursive() }

fibonacci(x I32) {
  recursive() + fibonacci(x - 1) + 1
}
