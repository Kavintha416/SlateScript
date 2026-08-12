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

### How to run .st Files

```bash
slate run (yourfile.st)
```

## Slattery UI

### Create a New Slattery Application

```bash
slate slattery new my_app
```

### Create UI Components

```st
import from "slattery" {Window, Column, Text, Button}

make App = Window {
    title: "My Application",
    Child: <Column> {
        Child: <Text> {
            value: "Welcome to Slattery!",
            Identity: welcome_text
        },
        Child: <Button> {
            label: "Click Me",
            on_click: handle_click
        }
    }
}

func handle_click<> {
    write("Button was clicked!")
}
```

### Styling(sts)

```sts
/* Component styles */
@Text {
    color: #1A1A1A;
    font-size: 18px;
}

@Button {
    background-color: #FF3B30;
    color: white;
    border-radius: 8px;
}

@Button:hover {
    background-color: #FF6B60;
}

/* ID-based styles */
#welcome_text {
    color: #FF3B30;
    font-size: 24px;
    font-weight: bold;
}

/* Class-based styles */
.highlight {
    background-color: yellow;
    font-weight: bold;
}

@Window {
    background-color: #FFFFFF;
}
```

Component properties

- title |	Window title | title: "My App"
- value |	Text content |	value: "Hello"
- label |	Button label |	label: "Submit"
- Identity | Component ID (for CSS) |	Identity: my_button
- on_click	| Click event handler |	on_click: my_function
- Child	Nested | child component |	Child: <Text> {...}
- children |	Array of children |	children: [<Text>, <Button>]

UI Components

- Window:	Main application window
- Column:	Vertical layout container
- Row:	Horizontal layout container
- Text:	Text label with styling support
- Button:	Clickable button with events
- Input:	Text input field

## Developer Tools

- Press | Ctrl+Shift+I | to view slattery dev tools

## SLIT Package Manager

### Install a package

```bash
slate slit install math-utils
```

### List installed packages

```bash
slate slit list
```

### Search for packages

```bash
slate slit search math
```

### Initialize package config

```bash
slate slit init
```

### Command sheet

- slate run <file>:	Execute a SlateScript file
- slate slattery new <name>:	Create a new Slattery app
- slate slit install <pkg>:	Install a package
- slate slit list:	List installed packages
- slate slit search [query]:	Search for packages
- slate slit init:	Initialize package config
- slate version:	Show version information
- slattery run <file>:	Run a Slattery app (standalone)
- slattery new <name>:	Create a Slattery app (standalone)

### License

This project is licensed under the MIT License - see the LICENSE file for details.
