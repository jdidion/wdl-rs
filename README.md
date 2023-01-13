# WDL for Rust

Rust library for Workflow Description Language (WDL).

Currently, there is an #[AST](src/ast.rs) and parsers based on both #[tree-sitter](src/parsers/tree_sitter/) and #[pest](src/parsers/pest/). There are plans to add a type-checker and expression evaluator.

## Example

```rust
use std::path::Path;
use wdl::{ast::VersionIdentifier, ast::DocumentElement, parsers::pest::PestParser};

fn main() {
    let wdl_path = Path::new("/path/to/workflow.wdl");
    let doc = PestParser::parse_file(wdl_path)?;
    assert_eq!(doc.version.identifier, VersionIdentifier::V1_1);
    for element in doc.body {
        match *element {
            DocumentElement::Import(i) => ...,
        }
    }
}
```