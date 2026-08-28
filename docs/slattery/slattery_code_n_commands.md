# Slattery(UI)

## Import UI Components
```st
import from "slattery" {Window, Column, Row, Text, Button, Input, Identity, Rewrite}
```

## Window

```st
Window {
    title: "My App",
    width: 480,
    height: 320,
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

- value - Text content

- Identity - ID for STS styling

- class - STS class

## Button

```st
Button {
    label: "Click Me",
    Identity: my_button,
    on_click: handle_click
}
```

## Input

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

## Full Example

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

## Commands

```cmd
# not needed you can run the normal slate command
slate slattery run main.st
```

```cmd
# to create a new app
slate slattery new app
```

```cmd
#to build an app
slate slattery build app
```

```
#to clean the app
slate slattery clean app
```