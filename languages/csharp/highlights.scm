;; Methods
(method_declaration name: (identifier) @function)
(local_function_statement name: (identifier) @function)

;; Types
(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
(namespace_declaration name: (identifier) @type)

(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)

[
  (implicit_type)
  (predefined_type)
] @type.builtin

(_ type: (identifier) @type)

;; Enum
(enum_member_declaration (identifier) @property)

;; Literals
[
  (real_literal)
  (integer_literal)
] @number

[
  (character_literal)
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
  (interpolation_start)
  (interpolation_quote)
 ] @string

[
  (boolean_literal)
  (null_literal)
] @constant

;; Comments
(comment) @comment

;; Tokens
[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "--"
  "-"
  "-="
  "&"
  "&="
  "&&"
  "+"
  "++"
  "+="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "|"
  "|="
  "||"
  "?"
  "??"
  "??="
  "^"
  "^="
  "~"
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  ":"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

;; Keywords
(modifier) @keyword
"this" @keyword
(escape_sequence) @string.escape

[
  "add"
  "alias"
  "as"
  "base"
  "break"
  "case"
  "catch"
  "checked"
  "class"
  "continue"
  "default"
  "delegate"
  "do"
  "else"
  "enum"
  "event"
  "explicit"
  "extern"
  "finally"
  "for"
  "foreach"
  "global"
  "goto"
  "if"
  "implicit"
  "interface"
  "is"
  "lock"
  "namespace"
  "notnull"
  "operator"
  "params"
  "return"
  "remove"
  "sizeof"
  "stackalloc"
  "static"
  "struct"
  "switch"
  "throw"
  "try"
  "typeof"
  "unchecked"
  "using"
  "while"
  "new"
  "await"
  "in"
  "yield"
  "get"
  "set"
  "when"
  "out"
  "ref"
  "from"
  "where"
  "select"
  "record"
  "init"
  "with"
  "let"
] @keyword

;; Class and inheritance
(base_list (identifier) @type) ;; applies to record_base too

;; Attribute
(attribute) @attribute

;; Parameter (declaration sites only)
(parameter name: (identifier) @variable)
(parameter (identifier) @variable)

;; Variable declarations (declaration sites only)
(variable_declarator (identifier) @variable)
(foreach_statement (identifier) @variable)
(catch_declaration (_) (identifier) @variable)

;; Method calls (function names only)
(invocation_expression (member_access_expression name: (identifier) @function))

;; Likely static calls: color qualifier as type when PascalCase
((invocation_expression 
   (member_access_expression 
     expression: (identifier) @type
     name: (identifier) @function))
  (#match? @type "^[A-Z][a-zA-Z0-9_]*$"))

;; Static calls on predefined types (e.g., string.Join)
(invocation_expression 
  (member_access_expression 
    expression: (predefined_type) @type.builtin
    name: (identifier) @function))
