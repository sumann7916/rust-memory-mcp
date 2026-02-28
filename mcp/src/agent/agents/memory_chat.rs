use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agent::message::{LLMResponse, Message, Role};
use crate::agent::tools::{GetMemoryIndexTool, SaveMemoryTool, SearchMemoryTool};
use crate::agent::traits::{Agent, Tool};
use crate::config::Config;
use crate::providers::EmbedderProvider;
use crate::qdrant::QdrantStore;
use crate::tools::{self, GetMemoryIndexParams, MemoryIndex, SearchMemoryParams};

pub struct MemoryChatAgent {
    user_id: String,
    embedder: Arc<Box<dyn EmbedderProvider>>,
    store: Arc<QdrantStore>,
    config: Arc<Config>,
    memory_index: Option<MemoryIndex>,
}

impl MemoryChatAgent {
    pub fn new(
        user_id: String,
        embedder: Arc<Box<dyn EmbedderProvider>>,
        store: Arc<QdrantStore>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            user_id,
            embedder,
            store,
            config,
            memory_index: None,
        }
    }

    pub async fn with_cached_index(mut self) -> Result<Self> {
        let params = crate::tools::GetMemoryIndexParams {
            user_id: self.user_id.clone(),
        };
        self.memory_index = Some(tools::get_memory_index(params, &self.store).await?);
        Ok(self)
    }

    fn build_system_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are a memory assistant. You MUST follow this exact order before every response:

1. Call get_memory_index(user_id) to get the list of available topics.
2. Match the user's query against that topic list — pick 2–5 relevant topics (e.g. words in their question that appear as topics, or related topics like mongodb for 'mongo', performance for 'slow').
3. Call search_memory with a descriptive query (the user's question + the matched topic names) and user_id. Example: query = \"how did I fix mongodb cpu spike monitoring\" + topics \"mongodb cpu performance\".
4. NEVER say \"I don't know\" or \"I don't have any memory\" without having done steps 1–3 first.
5. Only after you have the search results (and index from step 1), formulate your response from that data.

Your tools:
- get_memory_index(user_id): Returns topics, langs, repos, scopes, total, recent. Call this FIRST for every user message.
- search_memory(query, user_id, ...): Finds memories. Call this SECOND with a descriptive query that includes the user's question and 2–5 matched topics from step 1.
- save_memory(...): Use when the user shares something to remember.

If you see \"[Auto-retrieved context]\" in the messages, that is the result of steps 1–3 for this turn — use it to answer; do not say you have no memory.

Current user_id: "
        );
        prompt.push_str(&self.user_id);
        prompt.push_str("\n");

        if let Some(ref index) = self.memory_index {
            prompt.push_str("\n\nCached topics (refresh with get_memory_index): ");
            if index.topics.is_empty() {
                prompt.push_str("(none yet)");
            } else {
                prompt.push_str(&index.topics.join(", "));
            }
            prompt.push_str(&format!(" | Total memories: {}", index.total));
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
            )),
            Box::new(SaveMemoryTool::new(
                self.embedder.clone(),
                self.store.clone(),
                self.config.clone(),
            )),
            Box::new(GetMemoryIndexTool::new(self.store.clone())),
        ]
    }

    async fn before_llm(&self, messages: &mut Vec<Message>) -> Result<()> {
        let query = extract_last_user_message(messages);
        if query.is_empty() {
            return Ok(());
        }

        // Step 1: get_memory_index to get available topics
        let index_params = GetMemoryIndexParams {
            user_id: self.user_id.clone(),
        };
        let index = tools::get_memory_index(index_params, &self.store).await?;

        // Step 2: match user query against topic list — pick 2–5 relevant topics
        let matched_topics = match_topics_to_query(&query, &index.topics, 5);

        // Step 3: search_memory with descriptive query + matched topics
        let search_query = if matched_topics.is_empty() {
            query.clone()
        } else {
            format!("{} {}", query, matched_topics.join(" "))
        };
        let params = SearchMemoryParams {
            query: search_query,
            user_id: self.user_id.clone(),
            repo: None,
            module: None,
            lang: None,
            feature: None,
            limit: 5,
            min_score: Some(0.5),
        };

        let result = tools::search_memory(params, &**self.embedder, &self.store, &self.config).await?;

        if !result.results.is_empty() {
            let memory_context = format_memories_for_context(&result.results);
            inject_memory_context(messages, &memory_context);
        }

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

/// Pick up to `max` topics from `topic_list` that are relevant to `query` (substring or word overlap).
fn match_topics_to_query(query: &str, topic_list: &[String], max: usize) -> Vec<String> {
    let q_lower = query.to_lowercase();
    let q_words: std::collections::HashSet<_> = q_lower
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .collect();

    let mut matched = Vec::with_capacity(max);
    for topic in topic_list {
        if matched.len() >= max {
            break;
        }
        let t_lower = topic.to_lowercase();
        if q_lower.contains(&t_lower)
            || t_lower.contains(&q_lower)
            || q_words.contains(t_lower.as_str())
            || t_lower.split('-').any(|p| q_words.contains(&p))
        {
            if !matched.contains(topic) {
                matched.push(topic.clone());
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
    let mut context = String::from("Relevant memories:\n");
    for (i, mem) in memories.iter().enumerate() {
        context.push_str(&format!(
            "{}. [{}] {}\n",
            i + 1,
            mem.scope,
            mem.content
        ));
    }
    context
}

fn inject_memory_context(messages: &mut Vec<Message>, context: &str) {
    if let Some(pos) = messages.iter().rposition(|m| m.role == Role::User) {
        messages.insert(
            pos,
            Message::system(format!(
                "[Auto-retrieved context for the user's latest question — use this to answer.]\n{}",
                context
            )),
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
