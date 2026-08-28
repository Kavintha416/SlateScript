import from "slattery" {Window, Column, Text, Button, Identity, Rewrite}

make App = Window {
    title: "Test App",
    Child: <Column> {
        Child: <Text> {
            value: "Click the button!",
            Identity: status_text
        },
        
        Child: <Button> {
            label: "Click Me",
            on_click: handle_click,
            Identity: test_button
        }
    }
}

func handle_click<> {
    write("Button clicked!")  // This should appear in console
    Rewrite<status_text, "value", "Button was clicked!">
}