use crate::message::{Message, MessageId};
use crate::middleware::{Action, Middleware};
use anyhow::Result;
use serde_json::{json, Value};

pub struct ConfigurationMiddleware;

impl ConfigurationMiddleware {
    pub fn new() -> Self {
        Self
    }

    fn handle_configuration_request(&self, id: &MessageId, params: &Value) -> Option<Message> {
        let items = params.get("items")?.as_array()?;
        
        let mut responses = Vec::new();
        
        for item in items.iter() {
            let section = item.get("section")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            
            let response = match section {
                "csharp|symbol_search.dotnet_search_reference_assemblies" => json!(true),
                "visual_basic|symbol_search.dotnet_search_reference_assemblies" => json!(true),
                "navigation.dotnet_navigate_to_decompiled_sources" => json!(true),
                "navigation.dotnet_navigate_to_source_link_and_embedded_sources" => json!(true),
                "csharp|completion.dotnet_show_completion_items_from_unimported_namespaces" => json!(true),
                "visual_basic|completion.dotnet_show_completion_items_from_unimported_namespaces" => json!(true),
                "csharp|completion.dotnet_trigger_completion_in_argument_lists" => json!(true),
                "visual_basic|completion.dotnet_trigger_completion_in_argument_lists" => json!(true),
                "csharp|quick_info.dotnet_show_remarks_in_quick_info" => json!(true),
                "visual_basic|quick_info.dotnet_show_remarks_in_quick_info" => json!(true),
                "projects.dotnet_enable_automatic_restore" => json!(true),
                "projects.dotnet_enable_file_based_programs" => json!(true),
                "csharp|code_style.formatting.indentation_and_spacing.tab_width" => json!(4),
                "visual_basic|code_style.formatting.indentation_and_spacing.tab_width" => json!(4),
                "csharp|code_style.formatting.indentation_and_spacing.indent_size" => json!(4),
                "visual_basic|code_style.formatting.indentation_and_spacing.indent_size" => json!(4),
                "csharp|code_style.formatting.indentation_and_spacing.indent_style" => json!("space"),
                "visual_basic|code_style.formatting.indentation_and_spacing.indent_style" => json!("space"),
                "csharp|background_analysis.dotnet_analyzer_diagnostics_scope" => json!("openFiles"),
                "visual_basic|background_analysis.dotnet_analyzer_diagnostics_scope" => json!("openFiles"),
                "csharp|background_analysis.dotnet_compiler_diagnostics_scope" => json!("openFiles"),
                "visual_basic|background_analysis.dotnet_compiler_diagnostics_scope" => json!("openFiles"),
                "csharp|inlay_hints.dotnet_enable_inlay_hints_for_parameters" => json!(true),
                "visual_basic|inlay_hints.dotnet_enable_inlay_hints_for_parameters" => json!(true),
                "csharp|inlay_hints.dotnet_enable_inlay_hints_for_literal_parameters" => json!(true),
                "visual_basic|inlay_hints.dotnet_enable_inlay_hints_for_literal_parameters" => json!(true),
                "csharp|inlay_hints.dotnet_enable_inlay_hints_for_indexer_parameters" => json!(true),
                "visual_basic|inlay_hints.dotnet_enable_inlay_hints_for_indexer_parameters" => json!(true),
                "csharp|inlay_hints.dotnet_enable_inlay_hints_for_object_creation_parameters" => json!(true),
                "visual_basic|inlay_hints.dotnet_enable_inlay_hints_for_object_creation_parameters" => json!(true),
                "csharp|inlay_hints.dotnet_enable_inlay_hints_for_other_parameters" => json!(true),
                "visual_basic|inlay_hints.dotnet_enable_inlay_hints_for_other_parameters" => json!(true),
                "csharp|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_differ_only_by_suffix" => json!(false),
                "visual_basic|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_differ_only_by_suffix" => json!(false),
                "csharp|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_match_method_intent" => json!(false),
                "visual_basic|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_match_method_intent" => json!(false),
                "csharp|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_match_argument_name" => json!(false),
                "visual_basic|inlay_hints.dotnet_suppress_inlay_hints_for_parameters_that_match_argument_name" => json!(false),
                "csharp|inlay_hints.csharp_enable_inlay_hints_for_types" => json!(true),
                "visual_basic|inlay_hints.csharp_enable_inlay_hints_for_types" => json!(true),
                "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_variable_types" => json!(true),
                "visual_basic|inlay_hints.csharp_enable_inlay_hints_for_implicit_variable_types" => json!(true),
                "csharp|inlay_hints.csharp_enable_inlay_hints_for_lambda_parameter_types" => json!(true),
                "visual_basic|inlay_hints.csharp_enable_inlay_hints_for_lambda_parameter_types" => json!(true),
                "csharp|inlay_hints.csharp_enable_inlay_hints_for_implicit_object_creation" => json!(true),
                "visual_basic|inlay_hints.csharp_enable_inlay_hints_for_implicit_object_creation" => json!(true),
                "csharp|inlay_hints.csharp_enable_inlay_hints_for_collection_expressions" => json!(true),
                "visual_basic|inlay_hints.csharp_enable_inlay_hints_for_collection_expressions" => json!(true),
                _ => json!(null),
            };
            
            responses.push(response);
        }
        
        let response_msg = Message::Response(crate::message::ResponseMessage {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            result: Some(json!(responses)),
            error: None,
        });
        
        Some(response_msg)
    }
}

impl Middleware for ConfigurationMiddleware {
    fn name(&self) -> &str {
        "ConfigurationMiddleware"
    }

    fn process_server_message(&self, message: &Message) -> Result<Action> {
        if let Message::Request(ref req) = message {
            if req.method == "workspace/configuration" {
                if let Some(params) = &req.params {
                    if let Some(response) = self.handle_configuration_request(&req.id, params) {
                        return Ok(Action::Replace(response));
                    }
                }
            }
        }
        
        Ok(Action::Continue)
    }

    fn process_client_message(&self, _message: &Message) -> Result<Action> {
        Ok(Action::Continue)
    }
}
