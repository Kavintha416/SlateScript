use std::env;
use std::fs;

mod src {
    pub mod slattery {
        pub mod sla_lexer {
            pub use slate::slattery::sla_lexer::*;
        }
    }
}

fn main() {
    let source = r#"
make App = Window <title: "Test App">
make Text = Text <value: "Hello World">
make Button = Button <label: "Click Me">
render <App>
"#;
    
    let mut lexer = src::slattery::sla_lexer::UiLexer::new(source);
    let tokens = lexer.tokenize();
    
    println!("Tokens:");
    for (i, token) in tokens.iter().enumerate() {
        println!("{}: {:?}", i, token);
    }
}
