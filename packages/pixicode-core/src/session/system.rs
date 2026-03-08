//! System prompt builder — assembles system instructions for LLM calls.

/// Build a default system prompt for PixiCode sessions.
///
/// If `override_prompt` is provided it takes priority; otherwise a sensible
/// default is returned.  The prompt is kept short to preserve context window
/// budget — detailed instructions live in per-tool descriptions.
pub fn build_system_prompt(
    override_prompt: Option<&str>,
    working_dir: &str,
    available_tools: &[&str],
) -> String {
    if let Some(p) = override_prompt {
        if !p.is_empty() {
            return p.to_string();
        }
    }

    let tool_list = if available_tools.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nAvailable tools: {}",
            available_tools.join(", ")
        )
    };

    format!(
        "You are PixiCode, an AI coding assistant.\n\
         Working directory: {working_dir}\n\
         \n\
         Rules:\n\
         - Be concise and helpful.\n\
         - When asked to modify code, use the appropriate tool.\n\
         - Always read files before editing them.\n\
         - Prefer editing existing files over creating new ones.\n\
         - Run tests/builds when relevant.\n\
         - Explain your reasoning briefly.{tool_list}"
    )
}
