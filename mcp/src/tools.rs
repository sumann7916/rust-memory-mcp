use std::time::{SystemTime, UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::providers::EmbedderProvider;
use crate::qdrant::{confidence_from_count, MemoryPayload, QdrantStore};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SaveMemoryParams {
    pub content: String,
    pub user_id: String,
    #[serde(default)]
    pub topics: Vec<String>,
    pub scope: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub feature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SearchMemoryParams {
    pub query: String,
    pub user_id: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub feature: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub min_score: Option<f32>,
}

fn default_limit() -> usize {
    5
}

fn default_max_limit() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetAllMemoriesParams {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeleteMemoryParams {
    pub memory_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryItem {
    pub memory_id: String,
    pub content: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    pub timestamp: i64,
    pub last_accessed: i64,
    pub reinforcement_count: u32,
    pub confidence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boost: Option<f32>,
}

impl From<MemoryPayload> for MemoryItem {
    fn from(p: MemoryPayload) -> Self {
        Self {
            last_accessed: (p.last_accessed != 0)
                .then_some(p.last_accessed)
                .unwrap_or(p.timestamp),
            memory_id: p.memory_id,
            content: p.content,
            scope: p.scope,
            topics: p.topics,
            repo: p.repo,
            modules: p.modules,
            lang: p.lang,
            feature: p.feature,
            timestamp: p.timestamp,
            reinforcement_count: p.reinforcement_count,
            confidence: p.confidence,
            score: None,
            boost: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveResult {
    pub success: bool,
    pub message: String,
    pub memory: MemoryItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub results: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllMemoriesResult {
    pub memories: Vec<MemoryItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResult {
    pub success: bool,
    pub message: String,
}

const MERGE_MEMORIES_PROMPT: &str = r#"Merge these two memories into one concise sentence that captures both.
Memory 1: {existing}
Memory 2: {new}

Return only the merged sentence, nothing else."#;

const CONTRADICTION_CHECK_PROMPT: &str = r#"Does this new memory contradict any of the following existing memories?
Reply with ONLY the memory_id of the contradicted memory, or 'none' if there is no contradiction.

New memory: {new_memory}

Existing memories:
{existing_memories}"#;

/// Normalize a module name for consistent storage/search.
fn normalize_module_name(module: &str) -> String {
    let s = module.trim();
    let s = s.replace('\\', "/");
    let s = s.strip_prefix("./").unwrap_or(&s);
    let s = s.strip_prefix('/').unwrap_or(s);
    let s = s.strip_suffix('/').unwrap_or(s);
    s.to_string()
}

/// Validate scope against provided context.
fn validate_scope(scope: &str, repo: Option<&str>, lang: Option<&str>, module: Option<&str>) -> String {
    match scope {
        "global" => "global".to_string(),
        "lang" if lang.is_some() => "lang".to_string(),
        "feature" => "feature".to_string(),
        "repo" if repo.is_some() => "repo".to_string(),
        "module" if module.is_some() => "module".to_string(),
        _ => "global".to_string(),
    }
}

async fn handle_existing_memory_reinforcement(
    existing: &MemoryPayload,
    new_content: &str,
    new_topics: &[String],
    timestamp: i64,
    embedding: Vec<f32>,
    _embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
    user_id: &str,
) -> anyhow::Result<SaveResult> {
    let new_count = existing.reinforcement_count + 1;
    let new_confidence = confidence_from_count(new_count).to_string();

    let merged_content = if new_content != existing.content {
        format!("{} {}", existing.content, new_content)
    } else {
        existing.content.clone()
    };

    let merged_topics: Vec<String> = {
        let mut all_topics = existing.topics.clone();
        for topic in new_topics {
            if !all_topics.contains(topic) {
                all_topics.push(topic.clone());
            }
        }
        all_topics
    };

    let updated_payload = MemoryPayload {
        memory_id: existing.memory_id.clone(),
        content: merged_content.clone(),
        user_id: existing.user_id.clone(),
        scope: existing.scope.clone(),
        topics: merged_topics.clone(),
        repo: existing.repo.clone(),
        modules: existing.modules.clone(),
        lang: existing.lang.clone(),
        feature: existing.feature.clone(),
        timestamp: existing.timestamp,
        last_accessed: timestamp,
        reinforcement_count: new_count,
        confidence: new_confidence.clone(),
        superseded: false,
    };

    store
        .update_payload(&existing.memory_id, embedding, updated_payload)
        .await?;

    let memory_item = MemoryItem {
        memory_id: existing.memory_id.clone(),
        content: merged_content,
        scope: existing.scope.clone(),
        topics: merged_topics,
        repo: existing.repo.clone(),
        modules: existing.modules.clone(),
        lang: existing.lang.clone(),
        feature: existing.feature.clone(),
        timestamp: existing.timestamp,
        last_accessed: timestamp,
        reinforcement_count: new_count,
        confidence: new_confidence,
        score: None,
        boost: None,
    };

    Ok(SaveResult {
        success: true,
        message: format!("Memory reinforced (count: {}) for user {}", new_count, user_id),
        memory: memory_item,
    })
}

async fn handle_contradiction_check(
    potential_contradictions: &[&(MemoryPayload, f32)],
    _new_content: &str,
    timestamp: i64,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
) -> anyhow::Result<()> {
    if potential_contradictions.is_empty() {
        return Ok(());
    }

    for (contradicted, _) in potential_contradictions {
        let superseded_payload = MemoryPayload {
            superseded: true,
            last_accessed: timestamp,
            ..(*contradicted).clone()
        };

        let memory_id = superseded_payload.memory_id.clone();
        let old_embedding = embedder.embed(&superseded_payload.content).await?;
        store
            .update_payload(&memory_id, old_embedding, superseded_payload)
            .await?;
    }

    Ok(())
}

pub async fn save_memory(
    params: SaveMemoryParams,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
    config: &Config,
) -> anyhow::Result<SaveResult> {
    let validated_scope = validate_scope(
        &params.scope,
        params.repo.as_deref(),
        params.lang.as_deref(),
        params.module.as_deref(),
    );

    let modules = match params.module {
        Some(ref m) if validated_scope == "module" => {
            vec![normalize_module_name(m)]
        }
        _ => vec![],
    };

    let embedding = embedder.embed(&params.content).await?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let dedup_results = store
        .search_for_dedup(embedding.clone(), &params.user_id, 3)
        .await?;

    if let Some((existing, _)) = dedup_results
        .iter()
        .find(|(_, score)| *score >= config.memory_dedup_threshold)
    {
        return handle_existing_memory_reinforcement(
            existing,
            &params.content,
            &params.topics,
            timestamp,
            embedding,
            embedder,
            store,
            &params.user_id,
        )
        .await;
    }

    let contradiction_results = store
        .search_for_dedup(embedding.clone(), &params.user_id, 5)
        .await?;

    let potential_contradictions: Vec<_> = contradiction_results
        .iter()
        .filter(|(_, score)| *score >= 0.75 && *score < config.memory_dedup_threshold)
        .collect();

    handle_contradiction_check(
        &potential_contradictions,
        &params.content,
        timestamp,
        embedder,
        store,
    )
    .await?;

    let memory_id = Uuid::new_v4().to_string();

    let payload = MemoryPayload {
        memory_id: memory_id.clone(),
        content: params.content.clone(),
        user_id: params.user_id.clone(),
        scope: validated_scope.clone(),
        topics: params.topics.clone(),
        repo: params.repo.clone(),
        modules: modules.clone(),
        lang: params.lang.clone(),
        feature: params.feature.clone(),
        timestamp,
        last_accessed: timestamp,
        reinforcement_count: 1,
        confidence: "low".to_string(),
        superseded: false,
    };

    store.upsert(&memory_id, embedding, payload).await?;

    let memory_item = MemoryItem {
        memory_id,
        content: params.content,
        scope: validated_scope,
        topics: params.topics,
        repo: params.repo,
        modules,
        lang: params.lang,
        feature: params.feature,
        timestamp,
        last_accessed: timestamp,
        reinforcement_count: 1,
        confidence: "low".to_string(),
        score: None,
        boost: None,
    };

    Ok(SaveResult {
        success: true,
        message: format!("Memory saved for user {}", params.user_id),
        memory: memory_item,
    })
}

fn matches_module(memory: &MemoryPayload, search_module: Option<&String>) -> bool {
    let Some(search_mod) = search_module else {
        return memory.modules.is_empty();
    };
    
    if memory.modules.is_empty() {
        return true;
    }
    
    memory.modules.iter().any(|m| m == search_mod)
}

fn calculate_topic_overlap(memory: &MemoryPayload, search_topics: &[String]) -> f32 {
    if memory.topics.is_empty() || search_topics.is_empty() {
        return 0.0;
    }

    let matches = memory
        .topics
        .iter()
        .filter(|t| search_topics.contains(t))
        .count();

    matches as f32 / memory.topics.len().max(1) as f32
}

fn calculate_boost(
    memory: &MemoryPayload,
    repo: Option<&String>,
    module: Option<&String>,
    lang: Option<&String>,
    feature: Option<&String>,
    config: &Config,
) -> f32 {
    let mut boost = 1.0;

    match memory.scope.as_str() {
        "global" => boost *= 1.0,
        "lang" if memory.lang.as_ref() == lang => boost *= config.scope_boost_lang,
        "feature" if memory.feature.as_ref() == feature || (feature.is_some() && memory.feature.is_some()) => {
            boost *= config.scope_boost_feature
        }
        "repo" if memory.repo.as_ref() == repo => boost *= config.scope_boost_repo,
        "module" if matches_module(memory, module) => boost *= config.scope_boost_module,
        _ => boost *= 0.5,
    }

    let topic_overlap = if let Some(feature_name) = feature {
        calculate_topic_overlap(memory, &[feature_name.clone()])
    } else {
        0.0
    };
    boost *= 1.0 + topic_overlap * config.topic_boost_max;

    boost *= match memory.confidence.as_str() {
        "high" => 1.2,
        "medium" => 1.0,
        "low" => 0.8,
        _ => 1.0,
    };

    boost
}

fn compute_final_score(semantic_score: f32, boost: f32) -> f32 {
    semantic_score * boost
}

pub async fn search_memory(
    params: SearchMemoryParams,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
    config: &Config,
) -> anyhow::Result<SearchResult> {
    let embedding = embedder.embed(&params.query).await?;
    
    let search_limit = (params.limit * 3).min(30) as u64;

    let results = store
        .search(embedding, &params.user_id, search_limit)
        .await?;

    let base_threshold = params.min_score.unwrap_or(config.memory_score_threshold);

    let boosted_results: Vec<(MemoryPayload, f32, f32, f32)> = results
        .into_iter()
        .map(|(payload, semantic_score)| {
            let boost = calculate_boost(
                &payload,
                params.repo.as_ref(),
                params.module.as_ref(),
                params.lang.as_ref(),
                params.feature.as_ref(),
                config,
            );
            let final_score = compute_final_score(semantic_score, boost);
            (payload, semantic_score, boost, final_score)
        })
        .filter(|(_, _, _, final_score)| *final_score >= base_threshold)
        .collect();

    let mut sorted_results = boosted_results;
    sorted_results.sort_by(|(_, _, _, score1), (_, _, _, score2)| {
        score2.partial_cmp(score1).unwrap_or(std::cmp::Ordering::Equal)
    });

    sorted_results.truncate(params.limit.min(default_max_limit()));

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut items: Vec<MemoryItem> = Vec::new();
    for (payload, semantic_score, boost, _final_score) in sorted_results {
        let updated_payload = MemoryPayload {
            last_accessed: timestamp,
            ..payload
        };

        let embedding_for_update = embedder.embed(&updated_payload.content).await?;
        store
            .update_payload(&updated_payload.memory_id, embedding_for_update, updated_payload.clone())
            .await?;

        let mut item = MemoryItem::from(updated_payload);
        item.score = Some(semantic_score);
        item.boost = Some(boost);
        items.push(item);
    }

    Ok(SearchResult { results: items })
}

pub async fn get_all_memories(
    params: GetAllMemoriesParams,
    store: &QdrantStore,
) -> anyhow::Result<AllMemoriesResult> {
    let payloads = store.scroll(&params.user_id).await?;
    let memories: Vec<MemoryItem> = payloads.into_iter().map(MemoryItem::from).collect();
    Ok(AllMemoriesResult { memories })
}

pub async fn delete_memory(
    params: DeleteMemoryParams,
    store: &QdrantStore,
) -> anyhow::Result<DeleteResult> {
    store.delete(&params.memory_id, &params.user_id).await?;
    Ok(DeleteResult {
        success: true,
        message: format!("Memory {} deleted", params.memory_id),
    })
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CorrectMemoryParams {
    pub memory_id: String,
    pub user_id: String,
    pub correction: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CorrectResult {
    pub success: bool,
    pub message: String,
    pub old_memory_id: String,
    pub new_memory: MemoryItem,
}

pub async fn correct_memory(
    params: CorrectMemoryParams,
    embedder: &dyn EmbedderProvider,
    store: &QdrantStore,
) -> anyhow::Result<CorrectResult> {
    let existing = store
        .get_by_id(&params.memory_id, &params.user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Memory {} not found for user {}", params.memory_id, params.user_id))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let superseded = MemoryPayload {
        superseded: true,
        last_accessed: timestamp,
        ..existing.clone()
    };

    let old_memory_id = superseded.memory_id.clone();
    let old_embedding = embedder.embed(&superseded.content).await?;
    store
        .update_payload(&old_memory_id, old_embedding, superseded)
        .await?;

    let new_memory_id = Uuid::new_v4().to_string();
    let new_embedding = embedder.embed(&params.correction).await?;

    let new_topics = if params.topics.is_empty() {
        existing.topics.clone()
    } else {
        params.topics.clone()
    };

    let new_scope = params.scope.unwrap_or(existing.scope.clone());

    let new_payload = MemoryPayload {
        memory_id: new_memory_id.clone(),
        content: params.correction.clone(),
        user_id: params.user_id.clone(),
        scope: new_scope.clone(),
        topics: new_topics.clone(),
        repo: existing.repo.clone(),
        modules: existing.modules.clone(),
        lang: existing.lang.clone(),
        feature: existing.feature.clone(),
        timestamp,
        last_accessed: timestamp,
        reinforcement_count: 2,
        confidence: confidence_from_count(2).to_string(),
        superseded: false,
    };

    store
        .upsert(&new_memory_id, new_embedding, new_payload)
        .await?;

    let new_memory = MemoryItem {
        memory_id: new_memory_id,
        content: params.correction,
        scope: new_scope,
        topics: new_topics,
        repo: existing.repo,
        modules: existing.modules,
        lang: existing.lang,
        feature: existing.feature,
        timestamp,
        last_accessed: timestamp,
        reinforcement_count: 2,
        confidence: confidence_from_count(2).to_string(),
        score: None,
        boost: None,
    };

    Ok(CorrectResult {
        success: true,
        message: format!(
            "Memory {} superseded and corrected with new memory",
            params.memory_id
        ),
        old_memory_id: params.memory_id,
        new_memory,
    })
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetMemoryIndexParams {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryIndex {
    pub topics: Vec<String>,
    pub langs: Vec<String>,
    pub repos: Vec<String>,
    pub scopes: std::collections::HashMap<String, usize>,
    pub total: usize,
    pub recent: Vec<String>,
}

pub async fn get_memory_index(
    params: GetMemoryIndexParams,
    store: &QdrantStore,
) -> anyhow::Result<MemoryIndex> {
    let all_memories = store.scroll(&params.user_id).await?;

    let mut topics_set = std::collections::HashSet::new();
    let mut langs_set = std::collections::HashSet::new();
    let mut repos_set = std::collections::HashSet::new();
    let mut scopes_count = std::collections::HashMap::new();

    for memory in &all_memories {
        if memory.superseded {
            continue;
        }

        for topic in &memory.topics {
            topics_set.insert(topic.clone());
        }

        if let Some(ref lang) = memory.lang {
            langs_set.insert(lang.clone());
        }

        if let Some(ref repo) = memory.repo {
            repos_set.insert(repo.clone());
        }

        *scopes_count.entry(memory.scope.clone()).or_insert(0) += 1;
    }

    let mut topics: Vec<String> = topics_set.into_iter().collect();
    topics.sort();

    let mut langs: Vec<String> = langs_set.into_iter().collect();
    langs.sort();

    let mut repos: Vec<String> = repos_set.into_iter().collect();
    repos.sort();

    let recent: Vec<String> = {
        let mut non_superseded: Vec<_> = all_memories
            .iter()
            .filter(|m| !m.superseded)
            .collect();
        non_superseded.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        non_superseded
            .into_iter()
            .take(3)
            .map(|m| {
                let truncated = if m.content.len() > 60 {
                    format!("{}...", &m.content[..60])
                } else {
                    m.content.clone()
                };
                format!("[{}] {}", m.scope, truncated)
            })
            .collect()
    };

    let total = all_memories.iter().filter(|m| !m.superseded).count();

    Ok(MemoryIndex {
        topics,
        langs,
        repos,
        scopes: scopes_count,
        total,
        recent,
    })
}

