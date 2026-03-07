//! Web Search Tool — web search integration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Result title
    pub title: String,
    /// Result URL
    pub url: String,
    /// Snippet/description
    pub snippet: String,
    /// Source name
    pub source: Option<String>,
}

/// Parameters for the websearch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchParams {
    /// Search query
    pub query: String,
    /// Maximum number of results (default: 10)
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Search engine to use (default: google)
    #[serde(default)]
    pub engine: Option<String>,
    /// Language preference (e.g., "en", "zh")
    #[serde(default)]
    pub lang: Option<String>,
    /// Country/region (e.g., "us", "cn")
    #[serde(default)]
    pub region: Option<String>,
    /// Time range filter (e.g., "day", "week", "month", "year")
    #[serde(default)]
    pub time_range: Option<String>,
}

fn default_limit() -> u32 { 10 }

/// Tool for web search.
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "websearch"
    }

    fn description(&self) -> &'static str {
        "Search the web for information using various search engines"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["query".to_string()];
        schema.properties.insert("query".to_string(), ToolParameter::string("Search query"));
        schema.properties.insert("limit".to_string(), ToolParameter::integer("Maximum number of results"));
        schema.properties.insert("engine".to_string(), ToolParameter::string("Search engine (google, bing, duckduckgo)"));
        schema.properties.insert("lang".to_string(), ToolParameter::string("Language preference"));
        schema.properties.insert("region".to_string(), ToolParameter::string("Country/region"));
        schema.properties.insert("time_range".to_string(), ToolParameter::string("Time range filter"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: WebSearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let engine = params.engine.as_deref().unwrap_or("duckduckgo");
        
        // Perform search based on engine
        let results = match engine {
            "duckduckgo" | "ddg" => search_duckduckgo(&params).await?,
            "google" => search_google(&params).await?,
            "bing" => search_bing(&params).await?,
            _ => return Err(ToolError::InvalidParams(format!("Unknown search engine: {}", engine))),
        };

        // Format output
        let mut output = String::new();
        output.push_str(&format!("Query: {}\n", params.query));
        output.push_str(&format!("Engine: {}\n\n", engine));

        if results.is_empty() {
            output.push_str("No results found");
        } else {
            output.push_str(&format!("Found {} result(s):\n\n", results.len()));
            for (i, result) in results.iter().enumerate() {
                output.push_str(&format!("{}. **{}**\n", i + 1, result.title));
                output.push_str(&format!("   URL: {}\n", result.url));
                output.push_str(&format!("   {}\n\n", result.snippet));
            }
        }

        let result_data: Vec<serde_json::Value> = results.iter().map(|r| {
            serde_json::json!({
                "title": r.title,
                "url": r.url,
                "snippet": r.snippet,
                "source": r.source,
            })
        }).collect();

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "query": params.query,
            "engine": engine,
            "results": result_data,
            "count": results.len(),
        })))
    }
}

/// Search using DuckDuckGo (via HTML scraping).
async fn search_duckduckgo(params: &WebSearchParams) -> ToolResult<Vec<SearchResult>> {
    // DuckDuckGo HTML search
    let mut url = format!("https://html.duckduckgo.com/html/?q={}", 
        urlencoding::encode(&params.query));
    
    if let Some(region) = &params.region {
        url.push_str(&format!("&kl={}", region));
    }

    let client = reqwest::Client::new();
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; Pixicode/1.0)")
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("Search request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(ToolError::Execution(format!("Search returned status: {}", response.status())));
    }

    let html = response.text().await
        .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;

    parse_duckduckgo_results(&html, params.limit)
}

/// Search using Google (via scraping - in production use Custom Search API).
async fn search_google(params: &WebSearchParams) -> ToolResult<Vec<SearchResult>> {
    // Note: In production, use Google Custom Search JSON API
    // This is a simplified implementation
    
    let mut url = format!("https://www.google.com/search?q={}", 
        urlencoding::encode(&params.query));
    
    if let Some(lang) = &params.lang {
        url.push_str(&format!("&hl={}", lang));
    }
    if let Some(region) = &params.region {
        url.push_str(&format!("&gl={}", region));
    }

    let client = reqwest::Client::new();
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; Pixicode/1.0)")
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("Search request failed: {}", e)))?;

    let html = response.text().await
        .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;

    parse_google_results(&html, params.limit)
}

/// Search using Bing.
async fn search_bing(params: &WebSearchParams) -> ToolResult<Vec<SearchResult>> {
    let mut url = format!("https://www.bing.com/search?q={}", 
        urlencoding::encode(&params.query));
    
    if let Some(lang) = &params.lang {
        url.push_str(&format!("&cc={}", lang));
    }

    let client = reqwest::Client::new();
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; Pixicode/1.0)")
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("Search request failed: {}", e)))?;

    let html = response.text().await
        .map_err(|e| ToolError::Execution(format!("Failed to read response: {}", e)))?;

    parse_bing_results(&html, params.limit)
}

/// Parse DuckDuckGo HTML results.
fn parse_duckduckgo_results(html: &str, limit: u32) -> ToolResult<Vec<SearchResult>> {
    let mut results = Vec::new();
    
    // Simple HTML parsing (in production use proper HTML parser like scraper)
    let mut remaining = html;
    
    while let Some(result_start) = remaining.find("<a class=\"result__a\"") {
        if results.len() >= limit as usize {
            break;
        }

        // Extract URL
        if let Some(href_start) = remaining[result_start..].find("href=\"") {
            let href_pos = result_start + href_start + 6;
            if let Some(href_end) = remaining[href_pos..].find('"') {
                let url = decode_html_entities(&remaining[href_pos..href_pos + href_end]);
                
                // Extract title
                if let Some(title_start) = remaining[href_pos + href_end..].find('>') {
                    let title_pos = href_pos + href_end + title_start + 1;
                    if let Some(title_end) = remaining[title_pos..].find("</a>") {
                        let title = decode_html_entities(&remaining[title_pos..title_pos + title_end]);
                        
                        // Try to find snippet
                        let snippet = find_duckduckgo_snippet(&remaining[title_pos + title_end..]);
                        
                        results.push(SearchResult {
                            title,
                            url,
                            snippet: snippet.unwrap_or_default(),
                            source: None,
                        });
                        
                        remaining = &remaining[title_pos + title_end..];
                        continue;
                    }
                }
            }
        }
        remaining = &remaining[result_start + 1..];
    }

    Ok(results)
}

/// Parse Google HTML results.
fn parse_google_results(html: &str, limit: u32) -> ToolResult<Vec<SearchResult>> {
    let mut results = Vec::new();
    let mut remaining = html;
    
    // Look for search result containers
    while let Some(result_start) = remaining.find("<div class=\"g ") {
        if results.len() >= limit as usize {
            break;
        }

        // Find link
        if let Some(link_start) = remaining[result_start..].find("<a href=\"") {
            let href_pos = result_start + link_start + 10;
            if let Some(href_end) = remaining[href_pos..].find('"') {
                let url = decode_html_entities(&remaining[href_pos..href_pos + href_end]);
                
                // Find title (usually in h3)
                let title = if let Some(h3_start) = remaining[href_pos + href_end..].find("<h3>") {
                    let title_pos = href_pos + href_end + h3_start + 4;
                    if let Some(h3_end) = remaining[title_pos..].find("</h3>") {
                        decode_html_entities(&remaining[title_pos..title_pos + h3_end])
                    } else {
                        url.clone()
                    }
                } else {
                    url.clone()
                };
                
                // Find snippet
                let snippet = find_google_snippet(&remaining[href_pos + href_end..]);
                
                results.push(SearchResult {
                    title,
                    url,
                    snippet: snippet.unwrap_or_default(),
                    source: None,
                });
            }
        }
        remaining = &remaining[result_start + 1..];
    }

    Ok(results)
}

/// Parse Bing HTML results.
fn parse_bing_results(html: &str, limit: u32) -> ToolResult<Vec<SearchResult>> {
    // Similar parsing logic for Bing
    parse_google_results(html, limit) // Fallback to Google parser
}

/// Find snippet in DuckDuckGo results.
fn find_duckduckgo_snippet(html: &str) -> Option<String> {
    if let Some(snippet_start) = html.find("<a class=\"result__snippet\">") {
        let content_start = snippet_start + 29;
        if let Some(snippet_end) = html[content_start..].find("</a>") {
            return Some(decode_html_entities(&html[content_start..content_start + snippet_end]));
        }
    }
    None
}

/// Find snippet in Google results.
fn find_google_snippet(html: &str) -> Option<String> {
    if let Some(snippet_start) = html.find("<div class=\"IsZvec\")") {
        let content_start = snippet_start + 20;
        if let Some(snippet_end) = html[content_start..].find("</div>") {
            return Some(decode_html_entities(&html[content_start..content_start + snippet_end]));
        }
    }
    None
}

/// Decode HTML entities.
fn decode_html_entities(s: &str) -> String {
    let mut result = s.to_string();
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&#39;", "'");
    result = result.replace("&nbsp;", " ");
    result
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let bytes: Vec<u8> = s.chars()
            .flat_map(|c| {
                match c {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                        vec![c as u8]
                    }
                    ' ' => vec![b'+'],
                    _ => {
                        let mut bytes = vec![];
                        for byte in c.to_string().as_bytes() {
                            bytes.push(b'%');
                            bytes.push(HEX_TABLE[(byte >> 4) as usize]);
                            bytes.push(HEX_TABLE[(byte & 0x0F) as usize]);
                        }
                        bytes
                    }
                }
            })
            .collect();
        String::from_utf8_lossy(&bytes).to_string()
    }

    const HEX_TABLE: &[u8] = b"0123456789ABCDEF";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello+world");
        assert_eq!(urlencoding::encode("test@example"), "test%40example");
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("Hello &amp; World"), "Hello & World");
        assert_eq!(decode_html_entities("5 &lt; 10"), "5 < 10");
    }
}
