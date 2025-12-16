use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::env;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent";
const API_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CONFIDENCE: f32 = 50.0;
const HIGH_CONFIDENCE_THRESHOLD: f32 = 70.0;

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}

pub struct GeminiClient {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .context("GEMINI_API_KEY environment variable not set")?;
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECONDS))
            .build()?;
        
        Ok(Self { api_key, client })
    }

    pub fn is_available() -> bool {
        env::var("GEMINI_API_KEY").is_ok()
    }

    pub fn generate_content(&self, prompt: &str) -> Result<String> {
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
        };

        let url = format!("{}?key={}", GEMINI_API_URL, self.api_key);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Gemini API error ({}): {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response.json()
            .context("Failed to parse Gemini API response")?;

        gemini_response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .context("No response from Gemini API")
    }

    /// Analyze a file and suggest a category
    pub fn suggest_category(&self, file_path: &str, file_extension: &str, metadata: &str) -> Result<(String, f32)> {
        let prompt = format!(
            r#"You are a file organization assistant. Analyze the following file and suggest ONE appropriate category for organizing it.

File: {}
Extension: {}
Metadata: {}

Choose from these categories ONLY:
- Documents (for text documents, PDFs, spreadsheets, presentations)
- Images (for photos, pictures)
- Images/Screenshots (specifically for screenshots)
- Videos (for video files)
- Audio (for music and audio files)
- Archives (for compressed files)
- Code (for source code files)
- Data (for data files like CSV, databases)
- Executables (for executable files and installers)
- Other (for anything that doesn't fit)

Respond in this EXACT format on a single line:
CATEGORY: <category name> | CONFIDENCE: <0-100>

Example: CATEGORY: Images/Screenshots | CONFIDENCE: 95

Consider:
- If it's a PNG/JPG with dimensions suggesting widescreen (16:9), it's likely a screenshot
- Code files should be in Code category
- Documents include PDF, DOC, TXT, etc.
"#,
            file_path, file_extension, metadata
        );

        let response = self.generate_content(&prompt)?;
        
        // Parse response
        if let Some(category_line) = response.lines().find(|l| l.contains("CATEGORY:")) {
            let parts: Vec<&str> = category_line.split('|').collect();
            if parts.len() >= 2 {
                let category = parts[0]
                    .replace("CATEGORY:", "")
                    .trim()
                    .to_string();
                
                let confidence_part = parts[1]
                    .replace("CONFIDENCE:", "");
                let confidence_str = confidence_part.trim();
                
                let confidence: f32 = confidence_str.parse().unwrap_or(50.0);
                
                return Ok((category, confidence));
            }
        }
        
        // Fallback if parsing fails
        Ok(("Other".to_string(), DEFAULT_CONFIDENCE))
    }

    /// Ask user why the previous sorting was wrong and learn from it
    pub fn learn_from_correction(&self, file_path: &str, wrong_category: &str, correct_category: &str) -> Result<String> {
        let prompt = format!(
            r#"A file sorting system incorrectly categorized a file. Help understand why:

File: {}
Incorrect Category: {}
Correct Category: {}

Provide a brief explanation (2-3 sentences) of:
1. Why the file belongs in {} instead of {}
2. What characteristics distinguish files in the {} category

Keep your response concise and technical."#,
            file_path, wrong_category, correct_category,
            correct_category, wrong_category, correct_category
        );

        self.generate_content(&prompt)
    }

    /// Determine if AI should ask for help (low confidence)
    pub fn should_ask_user(confidence: f32) -> bool {
        confidence < HIGH_CONFIDENCE_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // This test just checks the function doesn't panic
        let _ = GeminiClient::is_available();
    }

    #[test]
    fn test_should_ask_user() {
        assert!(GeminiClient::should_ask_user(50.0));
        assert!(GeminiClient::should_ask_user(69.0));
        assert!(!GeminiClient::should_ask_user(HIGH_CONFIDENCE_THRESHOLD));
        assert!(!GeminiClient::should_ask_user(95.0));
    }
}
