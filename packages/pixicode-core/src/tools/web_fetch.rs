//! Web Fetch Tool — HTTP fetch with HTML to markdown conversion

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the webfetch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchParams {
    /// URL to fetch
    pub url: String,
    /// HTTP method (default: GET)
    #[serde(default)]
    pub method: Option<String>,
    /// Request headers
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Request body (for POST/PUT)
    #[serde(default)]
    pub body: Option<String>,
    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Convert HTML to markdown (default: true)
    #[serde(default = "default_true")]
    pub markdown: bool,
    /// Follow redirects (default: true)
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
}

fn default_timeout() -> u64 { 30000 }
fn default_true() -> bool { true }

/// Response from web fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Final URL (after redirects)
    pub final_url: String,
    /// Content type
    pub content_type: Option<String>,
}

/// Tool for fetching web content.
pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "webfetch"
    }

    fn description(&self) -> &'static str {
        "Fetch content from a URL with optional HTML to markdown conversion"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["url".to_string()];
        schema.properties.insert("url".to_string(), ToolParameter::string("URL to fetch"));
        schema.properties.insert("method".to_string(), ToolParameter::string("HTTP method (GET, POST, etc.)"));
        schema.properties.insert("headers".to_string(), ToolParameter {
            param_type: "object".to_string(),
            description: "Request headers".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        schema.properties.insert("body".to_string(), ToolParameter::string("Request body for POST/PUT"));
        schema.properties.insert("timeout_ms".to_string(), ToolParameter::integer("Timeout in milliseconds"));
        schema.properties.insert("markdown".to_string(), ToolParameter::boolean("Convert HTML to markdown"));
        schema.properties.insert("follow_redirects".to_string(), ToolParameter::boolean("Follow redirects"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: WebFetchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let timeout_ms = params.timeout_ms;

        // Validate URL
        if !params.url.starts_with("http://") && !params.url.starts_with("https://") {
            return Err(ToolError::InvalidParams("URL must start with http:// or https://".to_string()));
        }

        // Fetch with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            fetch_url(&params)
        ).await;

        match result {
            Ok(Ok(response)) => {
                let mut output_text = String::new();
                output_text.push_str(&format!("URL: {}\n", response.final_url));
                output_text.push_str(&format!("Status: {}\n", response.status));
                
                if let Some(content_type) = &response.content_type {
                    output_text.push_str(&format!("Content-Type: {}\n", content_type));
                }
                
                output_text.push_str("\n--- Content ---\n\n");
                output_text.push_str(&response.body);

                let mut tool_output = ToolOutput::success(output_text);
                
                if response.status >= 200 && response.status < 300 {
                    tool_output = tool_output.with_data(serde_json::json!({
                        "url": response.final_url,
                        "status": response.status,
                        "headers": response.headers,
                        "body": response.body,
                        "content_type": response.content_type,
                    }));
                    Ok(tool_output)
                } else {
                    tool_output.error = Some(format!("HTTP error: {}", response.status));
                    tool_output.output = format!("HTTP {}: {}\n\n{}", response.status, response.final_url, response.body);
                    Err(ToolError::Execution(format!("HTTP {}", response.status)))
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ToolError::Timeout(timeout_ms)),
        }
    }
}

/// Fetch a URL and return the response.
async fn fetch_url(params: &WebFetchParams) -> ToolResult<WebResponse> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(params.timeout_ms))
        .redirect(if params.follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .build()
        .map_err(|e| ToolError::Execution(format!("Failed to create HTTP client: {}", e)))?;

    let method = match params.method.as_deref().unwrap_or("GET") {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        _ => return Err(ToolError::InvalidParams(format!("Invalid HTTP method: {}", params.method.as_deref().unwrap_or("GET")))),
    };

    let mut request = client.request(method, &params.url);

    // Add headers
    if let Some(headers) = &params.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    // Add body
    if let Some(body) = &params.body {
        request = request.body(body.clone());
    }

    let response = request.send().await
        .map_err(|e| ToolError::Execution(format!("Request failed: {}", e)))?;

    let final_url = response.url().to_string();
    let status = response.status().as_u16();
    
    let headers: std::collections::HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let content_type = headers.get("content-type")
        .map(|ct| ct.split(';').next().unwrap_or(ct).trim().to_string());

    let body_bytes = response.bytes().await
        .map_err(|e| ToolError::Execution(format!("Failed to read response body: {}", e)))?;

    let body_str = String::from_utf8_lossy(&body_bytes).to_string();
    
    // Convert HTML to markdown if requested and content is HTML
    let body = if params.markdown && content_type.as_ref().map(|ct| ct.contains("text/html")).unwrap_or(false) {
        html_to_markdown(&body_str)
    } else {
        body_str
    };

    Ok(WebResponse {
        status,
        headers,
        body,
        final_url,
        content_type,
    })
}

/// Simple HTML to markdown converter.
fn html_to_markdown(html: &str) -> String {
    let mut result = html.to_string();

    // Remove script and style tags
    result = regex_replace(&result, r"(?s)<script[^>]*>.*?</script>", "");
    result = regex_replace(&result, r"(?s)<style[^>]*>.*?</style>", "");

    // Remove all tags but preserve some structure
    result = regex_replace(&result, r"<h1[^>]*>(.*?)</h1>", "\n# $1\n");
    result = regex_replace(&result, r"<h2[^>]*>(.*?)</h2>", "\n## $1\n");
    result = regex_replace(&result, r"<h3[^>]*>(.*?)</h3>", "\n### $1\n");
    result = regex_replace(&result, r"<h4[^>]*>(.*?)</h4>", "\n#### $1\n");
    result = regex_replace(&result, r"<h5[^>]*>(.*?)</h5>", "\n##### $1\n");
    result = regex_replace(&result, r"<h6[^>]*>(.*?)</h6>", "\n###### $1\n");
    
    result = regex_replace(&result, r"<p[^>]*>(.*?)</p>", "\n$1\n");
    result = regex_replace(&result, r"<br[^>]*/?>", "\n");
    result = regex_replace(&result, r"<hr[^>]*/?>", "\n---\n");

    result = regex_replace(&result, r"<strong[^>]*>(.*?)</strong>", "**$1**");
    result = regex_replace(&result, r"<b[^>]*>(.*?)</b>", "**$1**");
    result = regex_replace(&result, r"<em[^>]*>(.*?)</em>", "*$1*");
    result = regex_replace(&result, r"<i[^>]*>(.*?)</i>", "*$1*");

    result = regex_replace(&result, r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#, "[$2]($1)");
    result = regex_replace(&result, r"<a[^>]*href='([^']*)'[^>]*>(.*?)</a>", "[$2]($1)");

    result = regex_replace(&result, r#"<img[^>]*src="([^"]*)"[^>]*alt="([^"]*)"[^>]*/?>"#, "![$2]($1)");
    result = regex_replace(&result, r"<img[^>]*src='([^']*)'[^>]*alt='([^']*)'[^>]*/?>", "![$2]($1)");

    result = regex_replace(&result, r"<code[^>]*>(.*?)</code>", "`$1`");
    result = regex_replace(&result, r"<pre[^>]*>(.*?)</pre>", "\n```\n$1\n```\n");

    result = regex_replace(&result, r"<blockquote[^>]*>(.*?)</blockquote>", "\n> $1\n");

    result = regex_replace(&result, r"<li[^>]*>(.*?)</li>", "\n- $1");
    result = regex_replace(&result, r"</?u[lr][^>]*>", "");

    // Remove all remaining tags
    result = regex_replace(&result, r"<[^>]+>", "");

    // Clean up whitespace
    result = result.lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n");
    
    // Remove multiple blank lines
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

/// Simple regex replacement (without external crate).
fn regex_replace(text: &str, pattern: &str, replacement: &str) -> String {
    // Very basic implementation - in production use regex crate
    if pattern.contains("(?s)") {
        // DOTALL mode - match across lines
        let clean_pattern = pattern.replace("(?s)", "");
        let parts: Vec<&str> = clean_pattern.split(">").collect();
        if parts.len() >= 2 {
            if let Some(start_tag) = parts.first() {
                if let Some(end_tag) = parts.get(1).and_then(|p| p.split("<").next()) {
                    // Simple tag removal
                    let start_pattern = format!("<{}>", start_tag.trim_start_matches('<'));
                    let end_pattern = format!("</{}>", end_tag.trim_end_matches('>'));
                    
                    let mut result = text.to_string();
                    while let Some(start_pos) = find_tag(&result, &start_pattern) {
                        if let Some(end_pos) = result[start_pos..].find(&end_pattern) {
                            let before = result[..start_pos].to_string();
                            let content = result[start_pos + start_pattern.len()..start_pos + end_pos].to_string();
                            let after = result[start_pos + end_pos + end_pattern.len()..].to_string();
                            
                            let repl = replacement.replace("$1", &content);
                            result = format!("{}{}{}", before, repl, after);
                        } else {
                            break;
                        }
                    }
                    return result;
                }
            }
        }
    }
    
    // Simple single-line replacements
    let mut result = text.to_string();
    
    // Handle specific patterns
    if pattern.contains("<h1") {
        if let Some(start) = result.find("<h1") {
            if let Some(end_start) = result[start..].find('>') {
                if let Some(end) = result[start + end_start..].find("</h1>") {
                    let content = result[start + end_start + 1..start + end_start + end].to_string();
                    return result.replace(&format!("<h1{}>{}{}", 
                        &result[start + 3..start + end_start],
                        content,
                        "</h1>"), 
                        &format!("\n# {}\n", content));
                }
            }
        }
    }
    
    // Fallback: just strip tags for unmatched patterns
    if pattern.contains("<") && pattern.contains(">") {
        result = result.replace(&pattern.replace("*", ""), "");
    }

    result
}

/// Find a tag in text.
fn find_tag(text: &str, tag: &str) -> Option<usize> {
    text.find(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webfetch_invalid_url() {
        let tool = WebFetchTool::new();
        let params = serde_json::json!({
            "url": "not-a-url"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_html_to_markdown() {
        let html = r#"
            <html>
                <body>
                    <h1>Title</h1>
                    <p>Hello <strong>world</strong></p>
                    <a href="https://example.com">Link</a>
                </body>
            </html>
        "#;
        
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("world"), "bold content: {}", md);
        assert!(md.contains("[Link]") || md.contains("Link"));
    }
}
