(identifier) @variable

;; Enhanced contextual scope preparations for semantic token integration
;; These provide base scopes that will be refined by semantic tokens

;; Enhanced method scoping with contextual preparation
(method_declaration name: (identifier) @function.method)
(local_function_statement name: (identifier) @function.local)

;; Property declarations with enhanced scoping
(property_declaration name: (identifier) @property.definition)
(indexer_declaration) @property.indexer

;; Enhanced field and variable scoping
(field_declaration (variable_declaration (identifier) @property.field))
(event_field_declaration (variable_declaration (identifier) @function.event))

;; Constructor and destructor contextual scoping
(constructor_declaration name: (identifier) @function.constructor)
(destructor_declaration name: (identifier) @function.destructor)

;; Static member preparation (semantic tokens will add .static scope)
((method_declaration (modifier "static") name: (identifier)) @function.method.static)
((property_declaration (modifier "static") name: (identifier)) @property.definition.static)
((field_declaration (modifier "static")) @property.field.static)

;; Access modifier preparation
((method_declaration (modifier "public") name: (identifier)) @function.method.public)
((method_declaration (modifier "private") name: (identifier)) @function.method.private)
((method_declaration (modifier "protected") name: (identifier)) @function.method.protected)
((method_declaration (modifier "internal") name: (identifier)) @function.method.internal)

;; Virtual/override/abstract preparation
((method_declaration (modifier "virtual") name: (identifier)) @function.method.virtual)
((method_declaration (modifier "override") name: (identifier)) @function.method.override)
((method_declaration (modifier "abstract") name: (identifier)) @function.method.abstract)
((method_declaration (modifier "sealed") name: (identifier)) @function.method.sealed)

;; Async method preparation
((method_declaration (modifier "async") name: (identifier)) @function.method.async)

;; Readonly and const preparation
((field_declaration (modifier "readonly")) @property.field.readonly)
((field_declaration (modifier "const")) @property.field.const)

;; Generic type parameter scoping
(type_parameter (identifier) @type.parameter)
(type_parameter_constraint (identifier) @type.parameter.constraint)

;; Enhanced lambda and delegate scoping
(lambda_expression) @function.lambda
(anonymous_method_expression) @function.anonymous
(delegate_declaration name: (identifier) @type.delegate)

;; Namespace and using directive enhancements
(using_directive name: (identifier) @module.import)
(extern_alias_directive (identifier) @module.extern)

;; Pattern matching contextual scoping
(switch_expression_arm (declaration_pattern name: (identifier) @variable.pattern))
(switch_expression_arm (var_pattern (identifier) @variable.pattern))

;; LINQ query contextual preparation
(from_clause (identifier) @variable.query)
(let_clause (identifier) @variable.query)
(join_clause (identifier) @variable.query)

;; Exception handling contextual scoping
(catch_clause (catch_declaration type: (identifier) @type.exception))
(catch_clause (catch_declaration name: (identifier) @variable.exception))

;; Types

(interface_declaration name: (identifier) @type.interface)
(class_declaration name: (identifier) @type.class)
(enum_declaration name: (identifier) @type.enum)
(struct_declaration (identifier) @type.struct)
(record_declaration (identifier) @type.record)
(namespace_declaration name: (identifier) @module.namespace)

;; File-scoped namespaces (C# 10)
(file_scoped_namespace_declaration name: (identifier) @module.namespace.file_scoped)

;; Enhanced type contextual scoping
(generic_name (identifier) @type.generic)

;; Access modifier preparation for types
((class_declaration (modifier "public") name: (identifier)) @type.class.public)
((class_declaration (modifier "internal") name: (identifier)) @type.class.internal)
((class_declaration (modifier "private") name: (identifier)) @type.class.private)
((class_declaration (modifier "protected") name: (identifier)) @type.class.protected)

;; Class inheritance preparation
((class_declaration (modifier "abstract") name: (identifier)) @type.class.abstract)
((class_declaration (modifier "sealed") name: (identifier)) @type.class.sealed)
((class_declaration (modifier "static") name: (identifier)) @type.class.static)

;; Interface inheritance preparation
((interface_declaration (modifier "public") name: (identifier)) @type.interface.public)
((interface_declaration (modifier "internal") name: (identifier)) @type.interface.internal)

;; Struct modifiers
((struct_declaration (modifier) @_mod name: (identifier) @type.struct.readonly) (#eq? @_mod "readonly"))
((struct_declaration (modifier) @_mod name: (identifier) @type.struct.ref) (#eq? @_mod "ref"))

;; Record types (C# 9+)
((record_declaration (modifier "public") (identifier)) @type.record.public)
((record_declaration (modifier "internal") (identifier)) @type.record.internal)
(type_parameter (identifier) @property.definition)
(parameter type: (identifier) @type)
(type_argument_list (identifier) @type)
(as_expression right: (identifier) @type)
(is_expression right: (identifier) @type)

;; Pattern matching improvements (C# 8-11)
(declaration_pattern type: (identifier) @type name: (identifier) @variable)
(var_pattern (identifier) @variable)
(constant_pattern) @constant
(relational_pattern) @operator
(and_pattern) @operator
(or_pattern) @operator
(property_pattern_clause) @property
(tuple_pattern) @punctuation.bracket
(list_pattern) @punctuation.bracket
(parenthesized_pattern) @punctuation.bracket

(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)

(_ type: (identifier) @type)

(base_list (identifier) @type)

(predefined_type) @type.builtin

;; Switch expressions (C# 8)
(switch_expression) @keyword
(switch_expression_arm) @keyword

;; Nullable reference types (C# 8)
(nullable_type) @type

;; Enum
(enum_member_declaration (identifier) @property.definition)

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

(escape_sequence) @string.escape

[
  (boolean_literal)
  (null_literal)
] @constant.builtin

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
  (interpolation_brace)
]  @punctuation.bracket

;; Keywords

[
  (modifier)
  "this"
  (implicit_type)
] @keyword

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
  "required"
  "scoped"
] @keyword

;; Attribute

(attribute name: (identifier) @attribute)

;; Parameters

(parameter
  name: (identifier) @variable.parameter)

;; Enhanced parameter scoping with modifiers
((parameter (modifier) @_mod name: (identifier) @variable.parameter.ref) (#eq? @_mod "ref"))
((parameter (modifier) @_mod name: (identifier) @variable.parameter.out) (#eq? @_mod "out"))
((parameter (modifier) @_mod name: (identifier) @variable.parameter.in) (#eq? @_mod "in"))
((parameter (modifier) @_mod name: (identifier) @variable.parameter.params) (#eq? @_mod "params"))

;; Local variable enhanced scoping
(variable_declaration (identifier) @variable.local)
(declaration_expression (identifier) @variable.local)
(for_statement (variable_declaration (identifier)) @variable.local.loop)
(foreach_statement (identifier) @variable.local.loop)
(using_statement (variable_declaration (identifier)) @variable.local.using)

;; Enhanced local variable patterns
(tuple_element (identifier) @variable.local.tuple)
(discard) @variable.local.discard

;; Field vs local variable preparation (semantic tokens will override)
(field_declaration (variable_declaration (identifier) @property.definition))
(property_declaration name: (identifier) @property.definition)

;; Constant identification
(constant_pattern (identifier) @constant)
(local_declaration_statement (modifier) @keyword (variable_declaration (identifier) @constant) (#eq? @keyword "const"))

;; Event declarations
(event_declaration name: (identifier) @function.special)

;; Type constraints

(type_parameter_constraints_clause (identifier) @property.definition)

;; Method calls

;; Enhanced method invocation contextual scoping
(invocation_expression (member_access_expression name: (identifier) @function.method.invocation))
(invocation_expression (identifier) @function.invocation)

;; Static method calls (semantic tokens will add .static scope)
(member_access_expression 
  expression: (identifier) @type
  name: (identifier) @function.method.static_invocation)

;; Extension method calls
(invocation_expression 
  (member_access_expression 
    expression: _ 
    name: (identifier) @function.method.extension))

;; Constructor invocations
(object_creation_expression type: (identifier) @function.constructor.invocation)
(implicit_object_creation_expression) @function.constructor.invocation

;; Delegate invocations
(invocation_expression (identifier) @function.delegate.invocation)

;; Event invocations
(invocation_expression 
  (member_access_expression 
    name: (identifier) @function.event.invocation))

;; Async method invocations (await context)
(await_expression 
  (invocation_expression 
    (identifier) @function.method.async_invocation))

(await_expression 
  (invocation_expression 
    (member_access_expression 
      name: (identifier) @function.method.async_invocation)))

;; Error Highlighting Rules
;; These provide fallback highlighting for common error patterns
;; The diagnostics system will provide more specific error highlighting

;; Syntax errors - unmatched delimiters
(ERROR) @error

;; Missing semicolons or malformed statements
(_ (ERROR) @error)

;; Type errors and unknown identifiers (fallback)
((identifier) @error
  (#match? @error "^[A-Z][a-zA-Z0-9_]*$")  ; PascalCase that might be unresolved types
  (#not-has-parent? @error "type_argument_list" "generic_name" "base_list"))

;; Nullable reference type warnings (basic pattern detection)
(conditional_access_expression) @warning

;; Deprecated/obsolete members (attribute-based detection)
((method_declaration) @deprecated
  (#has-ancestor? @deprecated "attribute_list"))

((property_declaration) @deprecated  
  (#has-ancestor? @deprecated "attribute_list"))

((field_declaration) @deprecated
  (#has-ancestor? @deprecated "attribute_list"))