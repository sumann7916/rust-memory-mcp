use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::message::{LLMResponse, Message, Role, ToolDef};

#[async_trait]
pub trait AgentLLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>, tools: Option<&[ToolDef]>) -> Result<LLMResponse>;
}

// ============================================================================
// Ollama Provider
// ============================================================================

pub struct OllamaAgentProvider {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaAgentProvider {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Serialize, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Serialize, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: Value,
}

#[derive(Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

#[derive(Serialize)]
struct OllamaFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

fn convert_messages_to_ollama(messages: &[Message]) -> Vec<OllamaMessage> {
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            OllamaMessage {
                role: role.to_string(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|c| OllamaToolCall {
                            function: OllamaFunctionCall {
                                name: c.name.clone(),
                                arguments: c.arguments.clone(),
                            },
                        })
                        .collect()
                }),
            }
        })
        .collect()
}

fn convert_tools_to_ollama(tools: &[ToolDef]) -> Vec<OllamaTool> {
    tools
        .iter()
        .map(|t| OllamaTool {
            tool_type: "function".to_string(),
            function: OllamaFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
}

#[async_trait]
impl AgentLLMProvider for OllamaAgentProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Option<&[ToolDef]>) -> Result<LLMResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let ollama_messages = convert_messages_to_ollama(&messages);
        let ollama_tools = tools.map(convert_tools_to_ollama);

        let request = OllamaChatRequest {
            model: &self.model,
            messages: ollama_messages,
            stream: false,
            tools: ollama_tools,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<OllamaChatResponse>()
            .await?;

        if let Some(tool_calls) = response.message.tool_calls {
            if let Some(call) = tool_calls.into_iter().next() {
                return Ok(LLMResponse::tool_call(
                    uuid::Uuid::new_v4().to_string(),
                    call.function.name,
                    call.function.arguments,
                ));
            }
        }

        Ok(LLMResponse::text(response.message.content.trim()))
    }
}

// ============================================================================
// Anthropic Provider
// ============================================================================

pub struct AnthropicAgentProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicAgentProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicResponseBlock>,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: Value },
}

fn convert_messages_to_anthropic(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system_prompt: Option<String> = None;
    let mut anthropic_messages = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                match &mut system_prompt {
                    Some(existing) => {
                        existing.push_str("\n\n");
                        existing.push_str(&m.content);
                    }
                    None => system_prompt = Some(m.content.clone()),
                }
            }
            Role::User => {
                anthropic_messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicContent::Text(m.content.clone()),
                });
            }
            Role::Assistant => {
                if let Some(ref tool_calls) = m.tool_calls {
                    let blocks: Vec<AnthropicContentBlock> = tool_calls
                        .iter()
                        .map(|tc| AnthropicContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        })
                        .collect();
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Blocks(blocks),
                    });
                } else {
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Text(m.content.clone()),
                    });
                }
            }
            Role::Tool => {
                let block = AnthropicContentBlock::ToolResult {
                    tool_use_id: m.tool_call_id.clone().unwrap_or_default(),
                    content: m.content.clone(),
                };
                anthropic_messages.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicContent::Blocks(vec![block]),
                });
            }
        }
    }

    (system_prompt, anthropic_messages)
}

fn convert_tools_to_anthropic(tools: &[ToolDef]) -> Vec<AnthropicTool> {
    tools
        .iter()
        .map(|t| AnthropicTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.parameters.clone(),
        })
        .collect()
}

#[async_trait]
impl AgentLLMProvider for AnthropicAgentProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Option<&[ToolDef]>) -> Result<LLMResponse> {
        let (system, anthropic_messages) = convert_messages_to_anthropic(&messages);
        let anthropic_tools = tools.map(convert_tools_to_anthropic);

        let request = AnthropicRequest {
            model: &self.model,
            max_tokens: 4096,
            system,
            messages: anthropic_messages,
            tools: anthropic_tools,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<AnthropicResponse>()
            .await?;

        for block in response.content {
            match block {
                AnthropicResponseBlock::ToolUse { id, name, input } => {
                    return Ok(LLMResponse::tool_call(id, name, input));
                }
                AnthropicResponseBlock::Text { text } => {
                    if response.stop_reason.as_deref() != Some("tool_use") {
                        return Ok(LLMResponse::text(text.trim()));
                    }
                }
            }
        }

        Ok(LLMResponse::text(""))
    }
}

// ============================================================================
// Groq Provider (OpenAI-compatible)
// ============================================================================

pub struct GroqAgentProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GroqAgentProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct GroqRequest<'a> {
    model: &'a str,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GroqTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
}

#[derive(Serialize)]
struct GroqMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<GroqToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct GroqToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: GroqFunctionCall,
}

#[derive(Serialize, Deserialize)]
struct GroqFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct GroqTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: GroqFunction,
}

#[derive(Serialize)]
struct GroqFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
}

#[derive(Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
}

#[derive(Deserialize)]
struct GroqResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<GroqToolCall>>,
}

fn convert_messages_to_groq(messages: &[Message]) -> Vec<GroqMessage> {
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            GroqMessage {
                role: role.to_string(),
                content: if m.content.is_empty() {
                    None
                } else {
                    Some(m.content.clone())
                },
                tool_calls: m.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|c| GroqToolCall {
                            id: c.id.clone(),
                            call_type: "function".to_string(),
                            function: GroqFunctionCall {
                                name: c.name.clone(),
                                arguments: serde_json::to_string(&c.arguments).unwrap_or_default(),
                            },
                        })
                        .collect()
                }),
                tool_call_id: m.tool_call_id.clone(),
                name: m.name.clone(),
            }
        })
        .collect()
}

fn convert_tools_to_groq(tools: &[ToolDef]) -> Vec<GroqTool> {
    tools
        .iter()
        .map(|t| GroqTool {
            tool_type: "function".to_string(),
            function: GroqFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
}

#[async_trait]
impl AgentLLMProvider for GroqAgentProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Option<&[ToolDef]>) -> Result<LLMResponse> {
        let groq_messages = convert_messages_to_groq(&messages);
        let groq_tools = tools.map(convert_tools_to_groq);

        let request = GroqRequest {
            model: &self.model,
            messages: groq_messages,
            tools: groq_tools,
            tool_choice: tools.map(|_| "auto"),
        };

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<GroqResponse>()
            .await?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No response from Groq"))?;

        if let Some(tool_calls) = choice.message.tool_calls {
            if let Some(call) = tool_calls.into_iter().next() {
                let arguments: Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                return Ok(LLMResponse::tool_call(call.id, call.function.name, arguments));
            }
        }

        Ok(LLMResponse::text(
            choice.message.content.unwrap_or_default().trim(),
        ))
    }
}

// ============================================================================
// Gemini Provider (Google AI)
// ============================================================================

pub struct GeminiAgentProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiAgentProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    #[serde(default)]
    text: String,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiResponseFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiResponseFunctionCall {
    name: String,
    args: Value,
}

fn convert_messages_to_gemini(messages: &[Message]) -> (Option<String>, Vec<GeminiContentOwned>) {
    let mut system_instruction: Option<String> = None;
    let mut contents = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                match &mut system_instruction {
                    Some(existing) => {
                        existing.push_str("\n\n");
                        existing.push_str(&m.content);
                    }
                    None => system_instruction = Some(m.content.clone()),
                }
            }
            Role::User => {
                if m.content.is_empty() {
                    continue;
                }
                // GeminiContent needs owned parts; we'll build with owned strings
                contents.push(GeminiContentOwned {
                    role: "user".to_string(),
                    parts: vec![GeminiPartOwned::Text(m.content.clone())],
                });
            }
            Role::Assistant => {
                if let Some(ref tool_calls) = m.tool_calls {
                    let parts: Vec<GeminiPartOwned> = tool_calls
                        .iter()
                        .map(|tc| GeminiPartOwned::FunctionCall {
                            name: tc.name.clone(),
                            args: tc.arguments.clone(),
                        })
                        .collect();
                    if !parts.is_empty() {
                        contents.push(GeminiContentOwned {
                            role: "model".to_string(),
                            parts,
                        });
                    }
                } else if !m.content.is_empty() {
                    contents.push(GeminiContentOwned {
                        role: "model".to_string(),
                        parts: vec![GeminiPartOwned::Text(m.content.clone())],
                    });
                }
            }
            Role::Tool => {
                let name = m.name.clone().unwrap_or_default();
                let response: Value = serde_json::from_str(&m.content).unwrap_or(Value::Null);
                contents.push(GeminiContentOwned {
                    role: "user".to_string(),
                    parts: vec![GeminiPartOwned::FunctionResponse { name, response }],
                });
            }
        }
    }

    (system_instruction, contents)
}

// Owned variants for building request (Gemini API expects role on each content)
#[derive(Serialize)]
struct GeminiContentOwned {
    role: String,
    parts: Vec<GeminiPartOwned>,
}

/// Parts: { "text": "..." } | { "functionCall": { "name", "args" } } | { "functionResponse": { "name", "response" } }.
#[derive(Serialize)]
enum GeminiPartOwned {
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "functionCall")]
    FunctionCall { name: String, args: Value },
    #[serde(rename = "functionResponse")]
    FunctionResponse { name: String, response: Value },
}

#[derive(Serialize)]
struct GeminiRequestOwned {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    contents: Vec<GeminiContentOwned>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiToolsOwned>>,
}

#[derive(Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPartOwned>,
}

#[derive(Serialize)]
struct GeminiToolsOwned {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GeminiFunctionDeclOwned>,
}

#[derive(Serialize)]
struct GeminiFunctionDeclOwned {
    name: String,
    description: String,
    parameters: Value,
}

fn convert_tools_to_gemini(tools: &[ToolDef]) -> Vec<GeminiToolsOwned> {
    vec![GeminiToolsOwned {
        function_declarations: tools
            .iter()
            .map(|t| GeminiFunctionDeclOwned {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            })
            .collect(),
    }]
}

#[async_trait]
impl AgentLLMProvider for GeminiAgentProvider {
    async fn chat(&self, messages: Vec<Message>, tools: Option<&[ToolDef]>) -> Result<LLMResponse> {
        let (system_instruction, contents) = convert_messages_to_gemini(&messages);
        let tools_payload = tools.map(convert_tools_to_gemini);

        let system_instruction = system_instruction.map(|s| GeminiSystemInstruction {
            parts: vec![GeminiPartOwned::Text(s)],
        });

        let request = GeminiRequestOwned {
            system_instruction,
            contents,
            tools: tools_payload,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            let preview = body.chars().take(500).collect::<String>();
            anyhow::bail!("Gemini API error {}: {}", status, preview);
        }
        let gemini_response: GeminiResponse = serde_json::from_str(&body)?;

        let candidate = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| anyhow::anyhow!("No candidate in Gemini response"))?;

        for part in candidate.content.parts {
            if let Some(fc) = part.function_call {
                return Ok(LLMResponse::tool_call(
                    uuid::Uuid::new_v4().to_string(),
                    fc.name,
                    fc.args,
                ));
            }
            if !part.text.is_empty() {
                return Ok(LLMResponse::text(part.text.trim()));
            }
        }

        Ok(LLMResponse::text(""))
    }
}

// ============================================================================
// Factory
// ============================================================================

use crate::config::{Config, LLMProviderType};

pub fn create_agent_llm(config: &Config) -> Box<dyn AgentLLMProvider> {
    let provider_type = config.agent_provider.as_ref().unwrap_or(&config.llm_provider);
    let model = config.agent_model.clone().unwrap_or_else(|| {
        match provider_type {
            LLMProviderType::Gemini => "gemini-2.5-flash".to_string(),
            _ => config.llm_model().to_string(),
        }
    });

    match provider_type {
        LLMProviderType::Ollama => Box::new(OllamaAgentProvider::new(
            config.ollama_base_url.clone(),
            model,
        )),
        LLMProviderType::Anthropic => Box::new(AnthropicAgentProvider::new(
            config.anthropic_api_key.clone().expect("ANTHROPIC_API_KEY required"),
            model,
        )),
        LLMProviderType::Gemini => Box::new(GeminiAgentProvider::new(
            config.gemini_api_key.clone().expect("GEMINI_API_KEY required for agent"),
            model,
        )),
        LLMProviderType::OpenAI => Box::new(GroqAgentProvider::new(
            config.groq_api_key.clone().expect("GROQ_API_KEY required"),
            model,
        )),
    }
}
