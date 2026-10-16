# Slattery(UI)

## Import UI Components
```st
import from "slattery" {Window, Column, Row, Text, Button, Input}
```

## Window

```st
Window {
    title: "My App",
    Child: <Text> { value: "Hello" }
}
```

## Column

```st
Column {
    spacing: 10,
    Child: <Text> { value: "Item 1" },
    Child: <Text> { value: "Item 2" }
}
```

## Row

```st
Row {
    spacing: 10,
    Child: <Button> { label: "Left" },
    Child: <Button> { label: "Right" }
}
```

## Text

```st
Text {
    value: "Hello World",
    Identity: welcome_text
}
```

Properties:

- `value` — the text to display
- `Identity` — reserved for future styling

## Button

```st
Button {
    label: "Click Me",
    on_click: handle_click,
    arg: "optional string"
}
```

Properties:

- `label` — button text
- `on_click` — name of the function to call when clicked
- `arg` — an optional string passed as the first argument to the handler
- `Identity` — reserved for future styling

## Input

```st
Input {
    placeholder: "Enter text...",
    Identity: name_input,
    on_change: handle_input
}
```

- `value` — current value
- `placeholder` — placeholder text
- `on_change` — name of the function to call when the value changes
- `Identity` — reserved for future styling

## Styles

Declare styles at the top of the file with `style <Target> { ... }`.
Each block sets properties for a component type.

```st
style Window {
    background-color: #FFFFFF
}

style Text {
    color: #333333
    font-size: 18px
}

style Button {
    background-color: #FF3B30
    color: white
}
```

## Full Example

```st
// ─────────────────────────────────────────────────────────────────
//  Slattery Demo
// ─────────────────────────────────────────────────────────────────

style Window {
    background-color: #FFFFFF
}

style Text {
    color: #333333
    font-size: 18px
}

style Button {
    background-color: #FF3B30
    color: white
}

import from "slattery" {Window, Column, Row, Text, Button, Input}

make App = Window {
    title: "Slattery Demo",
    Child: <Column> {
        spacing: 16,
        Child: <Text> {
            value: "Welcome to Slattery!",
            Identity: title_text
        },
        Child: <Text> {
            value: "A SlateScript UI demonstration"
        },
        Child: <Row> {
            spacing: 12,
            Child: <Button> {
                label: "Say Hello",
                on_click: say_hello,
                arg: "Hello from a button!"
            },
            Child: <Button> {
                label: "Say Goodbye",
                on_click: say_hello,
                arg: "Goodbye!"
            }
        },
        Child: <Input> {
            placeholder: "Type something...",
            Identity: name_input,
            on_change: handle_input
        }
    }
}

func say_hello<message> {
    write(message)
}

func handle_input<text> {
    write("Input: " + text)
}
```

## Commands

```cmd
slattery run myapp
```

```cmd
# to create a new app
slattery new app
```
