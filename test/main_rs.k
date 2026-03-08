import(path String)   _
print (msg String)    _
flush (stdout Stdout) _
todo  ()              _

Stdin         _
Stdout        _
Runtime       (new () Runtime)
Arc           (new (state CompilerState) Arc)
CompilerState (default() CompilerState)

stdin  Stdin
stdout Stdout

arena    import("arena")
ast      import("ast")
compiler import("ast")
errors   import("ast")
file     import("ast")
lexer    import("ast")
lsp      import("ast")
parser   import("ast")
types    import("ast")

clear() {
    print("\x1B[2J\x1B[1;1H")
    flush(stdout)
}

live(filename String) {
    runtime = Runtime.new()
    state = Arc.new(CompilerState.default())
    filename = todo()
}
