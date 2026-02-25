use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    QueryPointsBuilder, ScrollPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn confidence_from_count(count: u32) -> &'static str {
    match count {
        1 => "low",
        2..=4 => "medium",
        _ => "high",
    }
}

fn default_reinforcement_count() -> u32 {
    1
}

fn default_confidence() -> String {
    "low".to_string()
}

fn default_scope() -> String {
    "global".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPayload {
    pub memory_id: String,
    pub content: String,
    pub user_id: String,
    #[serde(default = "default_scope")]
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
    #[serde(default)]
    pub last_accessed: i64,
    #[serde(default = "default_reinforcement_count")]
    pub reinforcement_count: u32,
    #[serde(default = "default_confidence")]
    pub confidence: String,
    #[serde(default)]
    pub superseded: bool,
}

pub struct QdrantStore {
    client: Qdrant,
    collection: String,
}

impl QdrantStore {
    pub async fn new(
        host: &str,
        port: u16,
        collection: String,
        vector_size: u64,
    ) -> anyhow::Result<Self> {
        let url = format!("http://{}:{}", host, port);
        let client = Qdrant::from_url(&url).build()?;

        let store = Self { client, collection };
        store.ensure_collection(vector_size).await?;
        Ok(store)
    }

    async fn ensure_collection(&self, vector_size: u64) -> anyhow::Result<()> {
        let collections = self.client.list_collections().await?;
        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if exists {
            return Ok(());
        }

        self.client
            .create_collection(
                CreateCollectionBuilder::new(&self.collection)
                    .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
            )
            .await?;

        Ok(())
    }

    pub async fn upsert(
        &self,
        id: &str,
        vector: Vec<f32>,
        payload: MemoryPayload,
    ) -> anyhow::Result<()> {
        let payload_map: HashMap<String, qdrant_client::qdrant::Value> =
            serde_json::from_value(serde_json::to_value(&payload)?)?;

        let point = PointStruct::new(id.to_string(), vector, payload_map);

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, vec![point]))
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        vector: Vec<f32>,
        user_id: &str,
        limit: u64,
    ) -> anyhow::Result<Vec<(MemoryPayload, f32)>> {
        let filter = Filter::must([
            Condition::matches("user_id", user_id.to_string()),
            Condition::matches("superseded", false),
        ]);

        let results = self
            .client
            .query(
                QueryPointsBuilder::new(&self.collection)
                    .query(vector)
                    .filter(filter)
                    .limit(limit)
                    .with_payload(true),
            )
            .await?;

        results
            .result
            .into_iter()
            .map(|point| {
                let payload_value = serde_json::to_value(&point.payload)?;
                let payload: MemoryPayload = serde_json::from_value(payload_value)?;
                Ok((payload, point.score))
            })
            .collect()
    }

    pub async fn search_for_dedup(
        &self,
        vector: Vec<f32>,
        user_id: &str,
        limit: u64,
    ) -> anyhow::Result<Vec<(MemoryPayload, f32)>> {
        let filter = Filter::must([
            Condition::matches("user_id", user_id.to_string()),
            Condition::matches("superseded", false),
        ]);

        let results = self
            .client
            .query(
                QueryPointsBuilder::new(&self.collection)
                    .query(vector)
                    .filter(filter)
                    .limit(limit)
                    .with_payload(true),
            )
            .await?;

        results
            .result
            .into_iter()
            .map(|point| {
                let payload_value = serde_json::to_value(&point.payload)?;
                let payload: MemoryPayload = serde_json::from_value(payload_value)?;
                Ok((payload, point.score))
            })
            .collect()
    }

    pub async fn get_by_id(
        &self,
        memory_id: &str,
        user_id: &str,
    ) -> anyhow::Result<Option<MemoryPayload>> {
        let filter = Filter::must([
            Condition::matches("memory_id", memory_id.to_string()),
            Condition::matches("user_id", user_id.to_string()),
        ]);

        let results = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection)
                    .filter(filter)
                    .limit(1)
                    .with_payload(true),
            )
            .await?;

        results
            .result
            .first()
            .map(|point| {
                let payload_value = serde_json::to_value(&point.payload)?;
                serde_json::from_value(payload_value).map_err(Into::into)
            })
            .transpose()
    }

    pub async fn update_payload(
        &self,
        memory_id: &str,
        vector: Vec<f32>,
        payload: MemoryPayload,
    ) -> anyhow::Result<()> {
        self.upsert(memory_id, vector, payload).await
    }

    pub async fn batch_update_last_accessed(
        &self,
        memory_ids: &[String],
        embeddings: &[(String, Vec<f32>)],
        payloads: Vec<MemoryPayload>,
    ) -> anyhow::Result<()> {
        if memory_ids.is_empty() {
            return Ok(());
        }

        let embedding_map: HashMap<&String, &Vec<f32>> =
            embeddings.iter().map(|(id, vec)| (id, vec)).collect();

        for payload in payloads {
            let memory_id = payload.memory_id.clone();
            let Some(vector) = embedding_map.get(&memory_id) else {
                continue;
            };
            self.upsert(&memory_id, (*vector).clone(), payload).await?;
        }

        Ok(())
    }

    pub async fn scroll(&self, user_id: &str) -> anyhow::Result<Vec<MemoryPayload>> {
        self.scroll_with_filter(user_id, false).await
    }

    pub async fn scroll_non_superseded(&self, user_id: &str) -> anyhow::Result<Vec<MemoryPayload>> {
        self.scroll_with_filter(user_id, true).await
    }

    async fn scroll_with_filter(
        &self,
        user_id: &str,
        exclude_superseded: bool,
    ) -> anyhow::Result<Vec<MemoryPayload>> {
        let base_conditions = vec![Condition::matches("user_id", user_id.to_string())];
        let superseded_conditions: Vec<_> = exclude_superseded
            .then(|| Condition::matches("superseded", false))
            .into_iter()
            .collect();

        let filter = Filter::must(
            base_conditions
                .into_iter()
                .chain(superseded_conditions)
                .collect::<Vec<_>>(),
        );

        let mut all_memories = Vec::new();
        let mut offset: Option<qdrant_client::qdrant::PointId> = None;

        loop {
            let builder = ScrollPointsBuilder::new(&self.collection)
                .filter(filter.clone())
                .limit(100)
                .with_payload(true);

            let builder = offset
                .as_ref()
                .map(|o| builder.clone().offset(o.clone()))
                .unwrap_or(builder);

            let result = self.client.scroll(builder).await?;

            let batch_memories: Result<Vec<_>, _> = result
                .result
                .iter()
                .map(|point| {
                    let payload_value = serde_json::to_value(&point.payload)?;
                    serde_json::from_value::<MemoryPayload>(payload_value).map_err(anyhow::Error::from)
                })
                .collect();

            all_memories.extend(batch_memories?);

            let Some(next_offset) = result.next_page_offset else {
                break;
            };
            offset = Some(next_offset);
        }

        Ok(all_memories)
    }

    pub async fn delete(&self, memory_id: &str, user_id: &str) -> anyhow::Result<bool> {
        let filter = Filter::must([
            Condition::matches("memory_id", memory_id.to_string()),
            Condition::matches("user_id", user_id.to_string()),
        ]);

        self.client
            .delete_points(DeletePointsBuilder::new(&self.collection).points(filter))
            .await?;

        Ok(true)
    }
}
