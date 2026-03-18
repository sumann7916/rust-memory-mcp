use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;

use crate::agent::llm::AgentLLMProvider;
use crate::agent::message::{LLMResponse, Message, Role};
use crate::agent::tools::{ConfigureSearchTool, GetMemoryIndexTool, SaveMemoryTool, SearchMemoryTool};
use crate::agent::traits::{Agent, Tool};
use crate::config::Config;
use crate::providers::EmbedderProvider;
use crate::qdrant::QdrantStore;
use crate::tools::{self, GetMemoryIndexParams, MemoryIndex};

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub min_score: Option<f32>,
    pub default_limit: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            min_score: Some(0.2),
            default_limit: 5,
        }
    }
}

pub struct MemoryChatAgent {
    user_id: String,
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
    memory_index: Option<MemoryIndex>,
    search_config: Arc<Mutex<SearchConfig>>,
    llm: Arc<Box<dyn AgentLLMProvider>>,
}

impl MemoryChatAgent {
    pub fn new(
        user_id: String,
        embedder: Arc<Box<dyn EmbedderProvider>>,
        store: Arc<QdrantStore>,
        config: Arc<Config>,
        llm: Arc<Box<dyn AgentLLMProvider>>,
    ) -> Self {
        Self {
            user_id,
            embedder,
            store,
            config,
            memory_index: None,
            search_config: Arc::new(Mutex::new(SearchConfig::default())),
            llm,
        }
    }

    pub async fn with_cached_index(mut self) -> Result<Self> {
        let params = crate::tools::GetMemoryIndexParams {
            user_id: self.user_id.clone(),
        };
        self.memory_index = Some(tools::get_memory_index(params, &self.store).await?);
        Ok(self)
    }

    pub fn search_config(&self) -> Arc<Mutex<SearchConfig>> {
        self.search_config.clone()
    }

    fn build_system_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are a memory assistant for user: "
        );
        prompt.push_str(&self.user_id);
        prompt.push_str("\n\n");
        prompt.push_str(
            "CRITICAL RULES:
1. You already know the user_id - it is shown above. NEVER ask for it.
2. ALWAYS call search_memory FIRST when user asks ANY question - even casual ones about code, repos, features, architecture, patterns, how things work, or what something is.
3. The ONLY time to skip search_memory is for pure greetings like 'hey' or 'hi' with nothing else.
4. If in doubt, SEARCH. It's better to search and find nothing than to miss relevant memories.

SEARCH TRIGGERS - Call search_memory when you see:
- Questions with 'what', 'how', 'why', 'where', 'tell me about', 'explain'
- Technical terms: quote, vendor, carrier, shipper, pricing, API, service, mongodb, etc.
- Feature names: marketplace, highway, auto-spot, award, etc.
- 'whats the link', 'relationship between', 'how does X work'

Your tools:
- search_memory: CALL THIS FIRST for any question. Required: query, user_id. Optional: repo, lang, module, feature.
- save_memory: Save new information when user shares something to remember.
  Required: content, user_id, scope
  Optional: topics (array), repo, lang, module, feature
  
  SCOPE DECISION TREE (pick the most specific that applies):
  - \"module\": Memory specific to a directory/module (requires: repo, module path)
  - \"repo\": Memory specific to a repository (requires: repo)
  - \"feature\": Memory about a product feature across repos (requires: feature name)
  - \"lang\": Memory about a programming language/tech (requires: lang)
  - \"global\": General preference or pattern that applies everywhere
  
  EXAMPLES:
  - User prefers Result over panic in Rust → scope=\"lang\", lang=\"rust\"
  - Quote links to Vendor in rust-mem repo → scope=\"repo\", repo=\"rust-mem\"
  - Auth module uses JWT → scope=\"module\", repo=\"myapp\", module=\"src/auth\"
  - Invoicing feature uses Stripe → scope=\"feature\", feature=\"invoicing\"
  - User prefers descriptive variable names → scope=\"global\"

- get_memory_index: List all topics/languages/repos in the memory store.
- configure_search: Adjust search sensitivity (min_score, limit).

How to respond:
- SEARCH FIRST, then synthesize results into a natural answer.
- Synthesize memories into conversational language - don't just list them.
- Keep it casual ('bro' style) - no formal business speak.
- If search returns nothing useful, say what you found (or didn't) and offer to save new info.
- When saving, ALWAYS use user_id=\""
        );
        prompt.push_str(&self.user_id);
        prompt.push_str("\".

Current user_id: ");
        prompt.push_str(&self.user_id);

        let config = self.search_config.lock().unwrap();
        prompt.push_str(&format!(
            "\nCurrent search settings: min_score={}, limit={}",
            config.min_score.unwrap_or(0.2),
            config.default_limit
        ));

        if let Some(ref index) = self.memory_index {
            prompt.push_str("\n\nMemory index: ");
            if index.topics.is_empty() {
                prompt.push_str("(no memories yet)");
            } else {
                prompt.push_str(&format!("{} memories with topics: {}", index.total, index.topics.join(", ")));
            }
        }

        prompt
    }
}

#[async_trait]
impl Agent for MemoryChatAgent {
    fn name(&self) -> &str {
        "memory-chat"
    }

    fn system_prompt(&self) -> String {
        self.build_system_prompt()
    }

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(SearchMemoryTool::new(
                self.embedder.clone(),
                self.store.clone(),
                self.config.clone(),
                self.search_config.clone(),
            )),
            Box::new(SaveMemoryTool::new(
                self.embedder.clone(),
                self.store.clone(),
                self.config.clone(),
            )),
            Box::new(GetMemoryIndexTool::new(self.store.clone())),
            Box::new(ConfigureSearchTool::new(self.search_config.clone())),
        ]
    }

    async fn before_llm(&self, messages: &mut Vec<Message>) -> Result<()> {
        // No auto-search anymore - LLM will call search_memory tool when needed
        Ok(())
    }

    async fn after_llm(&self, response: &mut LLMResponse) -> Result<()> {
        if let LLMResponse::Text(text) = response {
            let cleaned = clean_thinking_tags(text);
            *text = cleaned;
        }
        Ok(())
    }
}

#[derive(serde::Deserialize)]
struct QueryExpansionResponse {
    expanded_query: String,
    #[serde(default)]
    topics: Vec<String>,
}

async fn expand_query_with_llm(
    llm: &dyn AgentLLMProvider,
    user_query: &str,
    available_topics: &[String],
) -> Result<(String, Vec<String>)> {
    let topics_str = if available_topics.len() > 100 {
        available_topics[..100].join(", ")
    } else {
        available_topics.join(", ")
    };

    let prompt = format!(
        r#"Given the user's query and available memory topics, provide:
1. An expanded search query with synonyms and related terms
2. Up to 5 most relevant topics from the list

User query: "{user_query}"

Available topics: {topics_str}

Respond with ONLY valid JSON (no markdown, no explanation):
{{"expanded_query": "original query plus related terms", "topics": ["topic1", "topic2"]}}"#
    );

    let response = llm
        .chat(
            vec![
                Message::system("You expand search queries and select relevant topics. Output ONLY valid JSON."),
                Message::user(&prompt),
            ],
            None,
        )
        .await?;

    let text = match response {
        LLMResponse::Text(t) => t,
        _ => return Ok((user_query.to_string(), vec![])),
    };

    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    match serde_json::from_str::<QueryExpansionResponse>(cleaned) {
        Ok(parsed) => {
            let valid_topics: Vec<String> = parsed
                .topics
                .into_iter()
                .filter(|t| available_topics.contains(t))
                .take(5)
                .collect();
            Ok((parsed.expanded_query, valid_topics))
        }
        Err(e) => {
            eprintln!("[auto-search] Failed to parse LLM response: {}", e);
            eprintln!("[auto-search] Raw response: {}", cleaned);
            Ok((user_query.to_string(), vec![]))
        }
    }
}

/// Pick up to `max` topics from `topic_list` that are relevant to `query` (substring or word overlap).
#[allow(dead_code)]
fn match_topics_to_query(query: &str, topic_list: &[String], max: usize) -> Vec<String> {
    let q_lower = query.to_lowercase();
    let q_words: std::collections::HashSet<_> = q_lower
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .collect();

    // Normalize function to handle hyphens, underscores
    let normalize = |s: &str| s.replace('-', " ").replace('_', " ");
    let q_normalized = normalize(&q_lower);

    let mut matched = Vec::with_capacity(max);
    for topic in topic_list {
        if matched.len() >= max {
            break;
        }
        let t_lower = topic.to_lowercase();
        let t_normalized = normalize(&t_lower);
        
        // Direct matches
        if q_lower.contains(&t_lower) 
            || t_lower.contains(&q_lower)
            || q_words.contains(t_lower.as_str())
        {
            if !matched.contains(topic) {
                matched.push(topic.clone());
            }
            continue;
        }
        
        // Normalized matches (handles "highway service" → "highway-service")
        if q_normalized.contains(&t_normalized) || t_normalized.contains(&q_normalized) {
            if !matched.contains(topic) {
                matched.push(topic.clone());
            }
            continue;
        }
        
        // Word-part matches for hyphenated/compound words
        if t_lower.split('-').any(|p| q_words.contains(&p) || q_lower.contains(p))
            || t_lower.split('_').any(|p| q_words.contains(&p) || q_lower.contains(p))
        {
            if !matched.contains(topic) {
                matched.push(topic.clone());
            }
            continue;
        }
        
        // Prefix matching (e.g., "mongo" matches "mongodb")
        for word in &q_words {
            if t_lower.starts_with(word) || word.starts_with(&t_lower) {
                if !matched.contains(topic) {
                    matched.push(topic.clone());
                    break;
                }
            }
        }
    }
    matched
}

fn extract_last_user_message(messages: &[Message]) -> String {
    messages
        .iter()
        .rev()
        .find(|m| m.role == Role::User)
        .map(|m| m.content.clone())
        .unwrap_or_default()
}

fn format_memories_for_context(memories: &[crate::tools::MemoryItem]) -> String {
    let mut context = String::from("Search found ");
    context.push_str(&memories.len().to_string());
    context.push_str(" memories:\n");
    for (i, mem) in memories.iter().enumerate() {
        let score = mem.score.unwrap_or(0.0);
        context.push_str(&format!(
            "{}. [{}] (score: {:.2}) {}\n",
            i + 1,
            mem.scope,
            score,
            mem.content
        ));
    }
    context
}

fn inject_memory_context(messages: &mut Vec<Message>, context: &str) {
    if let Some(user_msg) = messages.iter_mut().rev().find(|m| m.role == Role::User) {
        user_msg.content = format!(
            "[Context from your memories:]\n{}\n\n[User's question:]\n{}",
            context,
            user_msg.content
        );
    }
}

fn clean_thinking_tags(text: &str) -> String {
    let mut result = text.to_string();
    
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result.find("</think>") {
            result = format!("{}{}", &result[..start], &result[end + 8..]);
        } else {
            break;
        }
    }
    
    result.trim().to_string()
}
