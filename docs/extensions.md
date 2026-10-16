# Extensions

Extensions add capabilities to SlateScript without modifying the core
language. Each extension is a Rust crate that implements the
`LanguageExtension` trait and registers itself with the parser at startup.

## What extensions can do

- **Declare keywords.** `slattery` claims `Window`, `Button`, etc. The
  parser emits a generic `Token::Identifier` for any unknown word; the
  extension registry resolves it to an extension-specific token.
- **Parse custom syntax.** An extension's `parse_extension_expression` and
  `parse_extension_statement` methods are called before core parsing at
  every site.
- **Handle custom expressions at runtime.** `interpret_extension` receives
  `Expression::Extension` nodes from the interpreter.
- **Post-process the AST.** `post_process_ast` runs after the full program
  is parsed.

## Available extensions

### `sfile`

Import functions from other `.st` files.

```st
import from "sfile" {functions.st}
```

### `slattery`

```st
import from "slattery" {Window, Column, Text, Button}
```

See the documentations files for more details
