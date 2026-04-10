//! Tree-sitter query patterns for function and type extraction per language.
//!
//! Each query uses two captures:
//!   @name — the identifier of the function/type
//!   @definition — the full node (whose text is extracted)
//!
//! We use sequential patterns (one per line) instead of `[...]` alternation
//! because alternation + outer `@definition` capture has inconsistent
//! behavior across tree-sitter grammar versions.

pub const RUST_FUNCTIONS: &str = r#"
(function_item
  name: (identifier) @name) @definition
"#;

pub const RUST_TYPES: &str = r#"
(struct_item
  name: (type_identifier) @name) @definition

(enum_item
  name: (type_identifier) @name) @definition

(type_item
  name: (type_identifier) @name) @definition
"#;

pub const GO_FUNCTIONS: &str = r#"
(function_declaration
  name: (identifier) @name) @definition
"#;

pub const GO_TYPES: &str = r#"
(type_declaration
  (type_spec
    name: (type_identifier) @name)) @definition
"#;

pub const PYTHON_FUNCTIONS: &str = r#"
(function_definition
  name: (identifier) @name) @definition
"#;

pub const PYTHON_TYPES: &str = r#"
(class_definition
  name: (identifier) @name) @definition
"#;

pub const TYPESCRIPT_FUNCTIONS: &str = r#"
(function_declaration
  name: (identifier) @name) @definition

(method_definition
  name: (property_identifier) @name) @definition
"#;

pub const TYPESCRIPT_TYPES: &str = r#"
(interface_declaration
  name: (type_identifier) @name) @definition

(type_alias_declaration
  name: (type_identifier) @name) @definition

(class_declaration
  name: (type_identifier) @name) @definition
"#;

pub const JAVASCRIPT_FUNCTIONS: &str = r#"
(function_declaration
  name: (identifier) @name) @definition

(method_definition
  name: (property_identifier) @name) @definition
"#;

pub const JAVASCRIPT_TYPES: &str = r#"
(class_declaration
  name: (identifier) @name) @definition
"#;
