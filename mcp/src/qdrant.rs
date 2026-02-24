use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
    QueryPointsBuilder, ScrollPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPayload {
    pub memory_id: String,
    pub content: String,
    pub user_id: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    pub timestamp: i64,
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

        if !exists {
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection)
                        .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
                )
                .await?;
        }

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
        repo: Option<&str>,
        module: Option<&str>,
        lang: Option<&str>,
        limit: u64,
    ) -> anyhow::Result<Vec<(MemoryPayload, f32)>> {
        let mut must_conditions = vec![Condition::matches("user_id", user_id.to_string())];

        let repo_filter = match repo {
            Some(r) => Filter::should([
                Condition::is_null("repo"),
                Condition::matches("repo", r.to_string()),
            ]),
            None => Filter::should([Condition::is_null("repo")]),
        };

        must_conditions.push(Condition::from(repo_filter));

        if let Some(l) = lang {
            must_conditions.push(Condition::matches("lang", l.to_string()));
        }

        // modules empty = repo-wide (applies to all modules)
        // modules populated = specific modules
        // When filtering by module: include (modules is empty OR modules contains provided)
        if let Some(m) = module {
            let module_filter = Filter::should([
                Condition::is_empty("modules"),
                Condition::matches("modules", m.to_string()),
            ]);
            must_conditions.push(Condition::from(module_filter));
        }

        let filter = Filter::must(must_conditions);

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

        let mut memories = Vec::new();
        for point in results.result {
            let score = point.score;
            let payload_value = serde_json::to_value(&point.payload)?;
            let payload: MemoryPayload = serde_json::from_value(payload_value)?;
            memories.push((payload, score));
        }

        Ok(memories)
    }

    pub async fn scroll(&self, user_id: &str) -> anyhow::Result<Vec<MemoryPayload>> {
        let filter = Filter::must([Condition::matches("user_id", user_id.to_string())]);

        let mut all_memories = Vec::new();
        let mut offset: Option<qdrant_client::qdrant::PointId> = None;

        loop {
            let mut builder = ScrollPointsBuilder::new(&self.collection)
                .filter(filter.clone())
                .limit(100)
                .with_payload(true);

            if let Some(ref o) = offset {
                builder = builder.offset(o.clone());
            }

            let result = self.client.scroll(builder).await?;

            for point in &result.result {
                let payload_value = serde_json::to_value(&point.payload)?;
                let payload: MemoryPayload = serde_json::from_value(payload_value)?;
                all_memories.push(payload);
            }

            match result.next_page_offset {
                Some(next_offset) => offset = Some(next_offset),
                None => break,
            }
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
