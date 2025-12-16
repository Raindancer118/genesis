use anyhow::{Result, Context};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use which::which;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent";
const API_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CONFIDENCE: f32 = 50.0;
const HIGH_CONFIDENCE_THRESHOLD: f32 = 70.0;
const MAX_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_DELAY_SECONDS: u64 = 20;
const MAX_RETRY_DELAY_SECONDS: u64 = 120;  // Cap exponential backoff at 2 minutes
const API_CALL_DELAY_SECONDS: u64 = 4; // 15 RPM = 4 seconds per request

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

#[derive(Debug, Deserialize)]
struct GeminiErrorResponse {
    error: GeminiError,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    code: u16,
    message: String,
    status: Option<String>,
    details: Option<Vec<ErrorDetail>>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[serde(rename = "@type")]
    error_type: String,
    #[serde(rename = "retryDelay")]
    retry_delay: Option<String>,
    #[serde(rename = "violations")]
    violations: Option<Vec<QuotaViolation>>,
}

#[derive(Debug, Deserialize)]
struct QuotaViolation {
    #[serde(rename = "quotaMetric")]
    quota_metric: Option<String>,
    #[serde(rename = "quotaId")]
    quota_id: Option<String>,
}

enum GeminiMode {
    Cli,
    Api {
        api_key: String,
        client: reqwest::blocking::Client,
        last_call_time: std::sync::Mutex<Option<Instant>>,
    },
}

pub struct GeminiClient {
    mode: GeminiMode,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        // Check if gemini CLI is available first
        if Self::is_cli_available() {
            return Ok(Self {
                mode: GeminiMode::Cli,
            });
        }
        
        // Fall back to API if CLI is not available
        let api_key = env::var("GEMINI_API_KEY")
            .context("GEMINI_API_KEY environment variable not set and gemini CLI not found")?;
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(API_TIMEOUT_SECONDS))
            .build()?;
        
        Ok(Self { 
            mode: GeminiMode::Api {
                api_key,
                client,
                last_call_time: std::sync::Mutex::new(None),
            }
        })
    }

    fn is_cli_available() -> bool {
        // Check if 'gemini' CLI tool is available in PATH
        which("gemini").is_ok()
    }

    pub fn is_available() -> bool {
        Self::is_cli_available() || env::var("GEMINI_API_KEY").is_ok()
    }

    pub fn generate_content(&self, prompt: &str) -> Result<String> {
        match &self.mode {
            GeminiMode::Cli => self.generate_content_cli(prompt),
            GeminiMode::Api { .. } => self.generate_content_api(prompt),
        }
    }

    fn generate_content_cli(&self, prompt: &str) -> Result<String> {
        // Use the gemini CLI to generate content
        // This assumes the CLI uses the command format: gemini generate <prompt>
        // NOTE: The prompt is passed as a command argument. Since we use Command::arg()
        // rather than shell execution, the argument is passed directly to the process
        // without shell interpretation, preventing command injection vulnerabilities.
        let output = Command::new("gemini")
            .arg("generate")
            .arg(prompt)
            .output()
            .context("Failed to execute gemini CLI command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Gemini CLI error: {}", stderr);
        }

        let stdout = String::from_utf8(output.stdout)
            .context("Failed to parse gemini CLI output as UTF-8")?;

        Ok(stdout.trim().to_string())
    }

    fn generate_content_api(&self, prompt: &str) -> Result<String> {
        if let GeminiMode::Api { last_call_time, .. } = &self.mode {
            // Rate limiting: wait between API calls
            match last_call_time.lock() {
                Ok(mut last_time) => {
                    if let Some(last) = *last_time {
                        let elapsed = last.elapsed();
                        let wait_duration = Duration::from_secs(API_CALL_DELAY_SECONDS);
                        if elapsed < wait_duration {
                            let sleep_duration = wait_duration - elapsed;
                            thread::sleep(sleep_duration);
                        }
                    }
                    *last_time = Some(Instant::now());
                }
                Err(poisoned) => {
                    // Mutex is poisoned, recover and apply rate limiting anyway
                    eprintln!("Warning: Rate limiting mutex was poisoned, recovering...");
                    let mut last_time = poisoned.into_inner();
                    if let Some(last) = *last_time {
                        let elapsed = last.elapsed();
                        let wait_duration = Duration::from_secs(API_CALL_DELAY_SECONDS);
                        if elapsed < wait_duration {
                            let sleep_duration = wait_duration - elapsed;
                            thread::sleep(sleep_duration);
                        }
                    }
                    *last_time = Some(Instant::now());
                }
            }
        }
        
        self.generate_content_with_retry(prompt, 0)
    }

    fn generate_content_with_retry(&self, prompt: &str, attempt: u32) -> Result<String> {
        let (api_key, client) = match &self.mode {
            GeminiMode::Api { api_key, client, .. } => (api_key, client),
            GeminiMode::Cli => anyhow::bail!("Cannot use API retry with CLI mode"),
        };

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
        };

        let url = format!("{}?key={}", GEMINI_API_URL, api_key);
        
        let response = client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            
            // Check if it's a rate limit error (429)
            if status.as_u16() == 429 {
                // Try to parse the error response to inspect details
                if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&error_text) {
                    // Check for Daily Quota Exceeded first
                    if Self::is_daily_quota_exceeded(&error_response) {
                        anyhow::bail!("Gemini Daily Quota Exceeded. Please try again tomorrow or upgrade your plan.");
                    }

                    if attempt < MAX_RETRY_ATTEMPTS {
                        let retry_delay = Self::extract_retry_delay(&error_response)
                            .unwrap_or_else(|| {
                                // Default exponential backoff if no delay provided or valid
                                DEFAULT_RETRY_DELAY_SECONDS
                                    .saturating_mul(2_u64.saturating_pow(attempt))
                                    .min(MAX_RETRY_DELAY_SECONDS)
                            });
                        
                        // Enforce a minimum delay if we are retrying, to avoid 0s loops
                        let final_delay = retry_delay.max(5);

                        eprintln!("{}", format!("Rate limit exceeded. Retrying in {} seconds... (attempt {}/{})", 
                            final_delay, attempt + 1, MAX_RETRY_ATTEMPTS).yellow());
                        
                        thread::sleep(Duration::from_secs(final_delay));
                        return self.generate_content_with_retry(prompt, attempt + 1);
                    }
                } else if attempt < MAX_RETRY_ATTEMPTS {
                     // Couldn't parse error, use exponential backoff
                     let retry_delay = DEFAULT_RETRY_DELAY_SECONDS
                        .saturating_mul(2_u64.saturating_pow(attempt))
                        .min(MAX_RETRY_DELAY_SECONDS);
                        
                     eprintln!("{}", format!("Rate limit exceeded. Retrying in {} seconds... (attempt {}/{})", 
                        retry_delay, attempt + 1, MAX_RETRY_ATTEMPTS).yellow());
                    
                    thread::sleep(Duration::from_secs(retry_delay));
                    return self.generate_content_with_retry(prompt, attempt + 1);
                }
            }
            
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

    fn extract_retry_delay(error_response: &GeminiErrorResponse) -> Option<u64> {
        if let Some(details) = &error_response.error.details {
            for detail in details {
                if detail.error_type == "type.googleapis.com/google.rpc.RetryInfo" {
                    if let Some(delay_str) = &detail.retry_delay {
                        // Parse delay string like "17s" or "17.390968484s"
                        if let Some(seconds_str) = delay_str.strip_suffix('s') {
                            if let Ok(seconds) = seconds_str.parse::<f64>() {
                                // Clamp to reasonable values and convert safely
                                let clamped = seconds.max(0.0).min(MAX_RETRY_DELAY_SECONDS as f64);
                                return Some(clamped.ceil() as u64);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn is_daily_quota_exceeded(error_response: &GeminiErrorResponse) -> bool {
         if let Some(details) = &error_response.error.details {
            for detail in details {
                if detail.error_type == "type.googleapis.com/google.rpc.QuotaFailure" {
                    if let Some(violations) = &detail.violations {
                        for violation in violations {
                            if let Some(id) = &violation.quota_id {
                                if id.contains("RequestsPerDay") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
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
