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
    border-radius: 8px
}

import from "slattery" {Window, Column, Text, Button, Identity, Rewrite}

make App = Window {
    title: "My Slattery App",
    Child: <Column> {
        Child: <Text> {
            value: "Welcome to your new App!"
        },
        Child: <Button> {
            label: "Click me!",
            on_click: main
        }
    }
}

func main<> {
    write("Hello World")
}
