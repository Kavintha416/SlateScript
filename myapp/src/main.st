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

import from "slattery" {Window, Column, Row, Text, Button}

make App = Window {
    title: "Style Test",
    Child: <Column> {
        spacing: 16,
        Child: <Text> {
            value: "This text uses the Text style",
            Identity: title_text
        },
        Child: <Row> {
            spacing: 10,
            Child: <Button> {
                label: "Red Button",
                on_click: on_red
            },
            Child: <Button> {
                label: "Also Red",
                on_click: on_red
            }
        }
    }
}

func on_red<> {
    write("Button clicked!")
}