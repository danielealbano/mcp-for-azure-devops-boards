use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Attribute macro to mark a function as an MCP tool.
///
/// This macro validates the required metadata and passes through the function.
/// The build script will scan for these attributes and generate the router code.
///
/// Usage:
/// ```rust
/// #[mcp_tool(
///     name = "azdo_list_iteration_paths",
///     description = "List iteration paths for a project or team"
/// )]
/// pub async fn list_iteration_paths(
///     client: &AzureDevOpsClient,
///     args: ListIterationPathsArgs,
/// ) -> Result<CallToolResult, McpError> {
///     // implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn mcp_tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Validate that we have the required attributes
    let attr_str = attr.to_string();
    if !attr_str.contains("name") || !attr_str.contains("description") {
        panic!("mcp_tool attribute requires both 'name' and 'description' parameters");
    }

    // Just pass through the function unchanged
    // The build script will scan for this attribute and generate the router
    TokenStream::from(quote! { #input_fn })
}
