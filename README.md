# SlateScript

**A small, extensible scripting language. UI, file imports, and
(eventually) machine learning are extensions, not built-ins.**

## Disclaimer

This project is in its beta stage expect lots of bugs!

## What works today

- Variables (`make`)
- Functions (`func name<a, b> { ... }` and calls `name<1, 2>`)
- Control flow (`if` / `elif` / `else`, `loop`)
- Console output (`write(...)`)
- Arithmetic and comparison operators
- Strings, integers, floats, booleans, null
- Arrays and objects
- String and array methods (`.len`, `.to_upper`, `.push`, `.get`, etc.)
- **Annotations** — `#[name]` that transforms the following statement
  - `#[struct]` — named struct values with field read/write
- **Extensions** — plug in new keywords and behavior
  - `sfile` — import functions from other `.st` files
  - `slattery` — native GUI applications via egui

## Installation

### From Source

```bash
git clone <repo>
cd SlateScript-main
cargo install --path .
```

Beware that this is an experimental release and most features are yet to come

## Making a new file

- To make a new slatescript file type the file name with the extension .st

### Print Statements

```st
write("Hello, World!")
```

### Variables

```st
make name = henry
make age = 20
```

### Functions

```st
func greet<name> {
    write("Hello, " + name + "!")
}

greet<"world">
```

### Control Flow

```st
make age = 20

if <age >= 18> {
    write("You are an adult")
} elif <age >= 13> {
    write("You are a teenager")
} else {
    write("You are a child")
}
```

### Loops

```st
loop 5 {
    write("Iteration")
}
```

### Expressions and Maths

```st
make result = 10 + 20 * 3
make is_equal = (5 + 5) == 10
make message = "Hello " + "World"
```

### loops with counters

```st
// Counted loop
loop 5 {
    write("Iteration")
}

// Loop with counter variable
loop 10 with i {
    write("Number: " + i)
}
```

### How to run .st Files

you can run using the executable by specifying the path

```bash
slate.exe myfile.st
```

or if you installed it to path use

```bash
slate myfile.st
```