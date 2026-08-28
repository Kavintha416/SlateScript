# SlateScript

**A modern, elegant programming language with built-in UI framework**

SlateScript is a versatile programming language that combines clean syntax with powerful UI capabilities. Write once, run everywhere - from console applications to native GUI apps with the built-in Slattery UI framework.

## Disclaimer

This project is in its beta stage expect lots of bugs!

## Features

- **Clean, Intuitive Syntax** - Simple and readable, designed for productivity
- **Built-in UI Framework (Slattery)** - Create native GUI applications using CSS-like styling
- **Package Manager (SLIT)** - Manage dependencies and extensions
- **Rich Type System** - Strings, integers, floats, and booleans
- **Control Flow** - `if`/`elif`/`else` statements and `loop` for iteration
- **Functions** - User-defined functions with parameters
- **Expression Evaluation** - Full arithmetic and comparison operators
- **CSS Styling** - Style your UI components with `.sts` files
- **Developer Tools** - Built-in devtools panel for debugging and inspection

## Installation

### From Source

```bash
git clone <repo>
cd slatescript
cargo build --release
sudo cp target/release/slate /usr/local/bin/
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

### i

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

```bash
slate run (yourfile.st)
```