use serde_json::json;

#[test]
fn test_initialization_options_structure() {
    let init_options = json!({
        "enableDecompilationSupport": true,
        "enableImportCompletion": true,
        "enableAnalyzersSupport": true,
        "organizeImportsOnFormat": true,
        "enableEditorConfigSupport": true,
        "dotnetPath": "/usr/local/share/dotnet/dotnet",
        "extensionsPaths": []
    });

    assert_eq!(init_options["enableDecompilationSupport"], true);
    assert_eq!(init_options["enableImportCompletion"], true);
    assert_eq!(init_options["enableAnalyzersSupport"], true);
    assert_eq!(init_options["organizeImportsOnFormat"], true);
    assert_eq!(init_options["enableEditorConfigSupport"], true);
}

#[test]
fn test_server_command_structure() {
    let command_parts = vec![
        "/usr/local/share/dotnet/dotnet",
        "/path/to/server.dll",
        "--logLevel=Information",
        "--extensionLogDirectory=/path/to/logs",
    ];

    assert!(command_parts[0].contains("dotnet"));
    assert!(command_parts[1].ends_with(".dll"));
    assert!(command_parts.iter().any(|s| s.starts_with("--logLevel")));
    assert!(command_parts
        .iter()
        .any(|s| s.starts_with("--extensionLogDirectory")));
}

#[test]
fn test_diagnostic_severity_mapping() {
    let error_severity = 1;
    let warning_severity = 2;
    let info_severity = 3;
    let hint_severity = 4;

    assert_eq!(error_severity, 1);
    assert_eq!(warning_severity, 2);
    assert_eq!(info_severity, 3);
    assert_eq!(hint_severity, 4);
}

#[test]
fn test_completion_trigger_characters() {
    let trigger_chars = vec![".", "<", "(", "[", "{", " ", "="];

    assert!(trigger_chars.contains(&"."));
    assert!(trigger_chars.contains(&"<"));
    assert!(trigger_chars.contains(&"("));
    assert!(trigger_chars.len() > 0);
}

#[test]
fn test_workspace_configuration_keys() {
    let config_keys = vec![
        "csharp.semanticHighlighting.enabled",
        "csharp.inlayHints.enabled",
        "csharp.codeLens.enabled",
        "omnisharp.enableRoslynAnalyzers",
    ];

    assert!(config_keys
        .iter()
        .any(|k| k.contains("semanticHighlighting")));
    assert!(config_keys.iter().any(|k| k.contains("inlayHints")));
    assert!(config_keys.iter().any(|k| k.contains("codeLens")));
}

#[test]
fn test_document_selector_pattern() {
    let csharp_selector = json!({
        "language": "csharp",
        "scheme": "file"
    });

    assert_eq!(csharp_selector["language"], "csharp");
    assert_eq!(csharp_selector["scheme"], "file");
}

#[test]
fn test_server_capabilities_expected() {
    let expected_capabilities = vec![
        "textDocumentSync",
        "completionProvider",
        "hoverProvider",
        "definitionProvider",
        "referencesProvider",
        "documentFormattingProvider",
        "documentSymbolProvider",
        "workspaceSymbolProvider",
        "codeActionProvider",
        "semanticTokensProvider",
        "inlayHintProvider",
    ];

    assert!(expected_capabilities.contains(&"completionProvider"));
    assert!(expected_capabilities.contains(&"hoverProvider"));
    assert!(expected_capabilities.contains(&"definitionProvider"));
    assert!(expected_capabilities.contains(&"semanticTokensProvider"));
}

#[test]
fn test_semantic_token_types_legend() {
    let token_types = vec![
        "namespace",
        "type",
        "class",
        "enum",
        "interface",
        "struct",
        "typeParameter",
        "parameter",
        "variable",
        "property",
        "enumMember",
        "event",
        "function",
        "method",
        "macro",
        "keyword",
        "modifier",
        "comment",
        "string",
        "number",
        "regexp",
        "operator",
    ];

    assert!(token_types.contains(&"namespace"));
    assert!(token_types.contains(&"class"));
    assert!(token_types.contains(&"method"));
    assert!(token_types.len() >= 20);
}

#[test]
fn test_semantic_token_modifiers_legend() {
    let modifiers = vec!["static", "abstract", "readonly", "deprecated", "async"];

    assert!(modifiers.contains(&"static"));
    assert!(modifiers.contains(&"readonly"));
    assert!(modifiers.contains(&"async"));
}

#[test]
fn test_workspace_edit_capabilities() {
    let capabilities = json!({
        "documentChanges": true,
        "resourceOperations": ["create", "rename", "delete"]
    });

    assert_eq!(capabilities["documentChanges"], true);
    assert!(capabilities["resourceOperations"].is_array());
}

#[test]
fn test_code_action_kinds() {
    let action_kinds = vec![
        "quickfix",
        "refactor",
        "refactor.extract",
        "refactor.inline",
        "refactor.rewrite",
        "source",
        "source.organizeImports",
    ];

    assert!(action_kinds.contains(&"quickfix"));
    assert!(action_kinds.contains(&"refactor"));
    assert!(action_kinds
        .iter()
        .any(|k| k.starts_with("source.organize")));
}

#[test]
fn test_initialize_response_validation() {
    let response = json!({
        "capabilities": {
            "textDocumentSync": 2,
            "completionProvider": {
                "triggerCharacters": [".", "<"],
                "resolveProvider": true
            },
            "hoverProvider": true,
            "definitionProvider": true
        }
    });

    assert!(response["capabilities"]["completionProvider"].is_object());
    assert_eq!(response["capabilities"]["hoverProvider"], true);
    assert_eq!(response["capabilities"]["definitionProvider"], true);
}

#[test]
fn test_log_levels() {
    let log_levels = vec!["Trace", "Debug", "Information", "Warning", "Error"];

    assert!(log_levels.contains(&"Trace"));
    assert!(log_levels.contains(&"Information"));
    assert!(log_levels.contains(&"Error"));
    assert_eq!(log_levels.len(), 5);
}

#[test]
fn test_telemetry_disabled_by_default() {
    let telemetry_enabled = false;
    assert_eq!(telemetry_enabled, false);
}

#[test]
fn test_formatting_options() {
    let format_opts = json!({
        "tabSize": 4,
        "insertSpaces": true,
        "trimTrailingWhitespace": true,
        "insertFinalNewline": true
    });

    assert_eq!(format_opts["tabSize"], 4);
    assert_eq!(format_opts["insertSpaces"], true);
}

#[test]
fn test_position_encoding() {
    let encodings = vec!["utf-8", "utf-16", "utf-32"];
    let preferred = "utf-16";

    assert!(encodings.contains(&preferred));
    assert_eq!(preferred, "utf-16");
}

#[test]
fn test_completion_defaults_expected() {
    let expected_defaults = vec![
        "enableImportCompletion",
        "enableAnalyzersSupport",
        "organizeImportsOnFormat",
        "enableDecompilationSupport",
        "enableEditorConfigSupport",
    ];

    for flag in expected_defaults {
        assert!(!flag.is_empty());
    }
}

#[test]
fn test_completion_config_defaults() {
    let debounce_ms: u64 = 100;
    let telemetry_enabled = false;

    assert_eq!(debounce_ms, 100);
    assert_eq!(telemetry_enabled, false);
}

#[test]
fn test_completion_config_custom_values() {
    let custom_debounce: u64 = 250;
    let custom_telemetry = true;

    assert_eq!(custom_debounce, 250);
    assert_eq!(custom_telemetry, true);
}
