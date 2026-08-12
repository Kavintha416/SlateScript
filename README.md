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
write("hello")
```

### Variables

```st
make name = henry
make age = 20
```

### Functions

```st
func greeting<name: string> {
  write("hello, " + name)
}

greeting<"world">
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
## Extensions

### Slattery(UI)

Slattery is a UI framework extension for SlateScript. It adds native GUI capabilities using the egui library. The extension system allows Slattery to hook into the language without modifying the core parser/interpreter.

#### Import UI Components

import from "slattery" {Window, Column, Row, Text, Button, Input, Identity, Rewrite}

#### Window

```st
Window {
    title: "My App",
    width: 480,
    height: 320,
    Child: <Text> { value: "Hello" }
}
```

#### Column

```st
Column {
    spacing: 10,
    Child: <Text> { value: "Item 1" },
    Child: <Text> { value: "Item 2" }
}
```

#### Row

```st
Row {
    spacing: 10,
    Child: <Button> { label: "Left" },
    Child: <Button> { label: "Right" }
}
```

#### Text

```st
Text {
    value: "Hello World",
    Identity: welcome_text
}
```

Properties:

- value - Text content

- Identity - ID for STS styling

- class - STS class

#### Button

```st
Button {
    label: "Click Me",
    Identity: my_button,
    on_click: handle_click
}
```

#### Input

```st
Input {
    placeholder: "Enter text...",
    Identity: name_input,
    on_change: handle_input
}
```

- value - Current value

- placeholder - Placeholder text

- Identity - ID for CSS styling

- on_change - Called when value changes

#### Full Example

```st
import from "slattery" {Window, Column, Text, Button, Input, Identity, Rewrite}

make App = Window {
    title: "Slattery Demo",
    Child: <Column> {
        spacing: 16,
        
        Child: <Text> {
            value: "Welcome to Slattery!",
            Identity: title_text
        },
        
        Child: <Text> {
            value: "Enter your name:",
            Identity: prompt_text
        },
        
        Child: <Input> {
            placeholder: "Your name...",
            Identity: name_input,
            on_change: handle_name
        },
        
        Child: <Button> {
            label: "Say Hello",
            on_click: say_hello
        },
        
        Child: <Button> {
            label: "Reset",
            on_click: reset
        }
    }
}

func handle_name<name> {
    // Store the name
}

func say_hello<> {
    // Use Rewrite to update the title
    Rewrite<title_text, "value", "Hello, " + name_input>
}

func reset<> {
    Rewrite<title_text, "value", "Welcome to Slattery!">
}
```

#### Commands

```cmd
slate slattery run main.st
```

```cmd
slate slattery new app
```

```cmd
slate slattery build app
```

```
slate slattery clean app
```