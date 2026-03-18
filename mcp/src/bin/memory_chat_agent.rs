//! Local runner for the memory-chat agent.
//!
//! Run with Gemini:
//!   GEMINI_API_KEY=your_key \
//!   LLM_PROVIDER=gemini \
//!   AGENT_PROVIDER=gemini \
//!   USER_ID=sumankhadka \
//!   cargo run --bin memory_chat_agent
//!
//! Requires Qdrant running (e.g. docker run -p 6334:6334 qdrant/qdrant).

use std::io::{self, BufRead};
use std::sync::Arc;

use memory_mcp::agent::{create_agent_llm, AgentRuntime, MemoryChatAgent, Message};
use memory_mcp::config::Config;
use memory_mcp::providers::create_embedder;
use memory_mcp::qdrant::QdrantStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let user_id = std::env::var("USER_ID").unwrap_or_else(|_| "sumankhadka".to_string());

    let embedder = create_embedder(&config);
    let store = QdrantStore::new(
        &config.qdrant_host,
        config.qdrant_port,
        config.qdrant_collection.clone(),
        config.vector_size() as u64,
    )
    .await?;

    let llm = create_agent_llm(&config);

    // Readiness check: one no-tools call to verify API key and model work
    let ping = llm
        .chat(
            vec![
                Message::system("You are a test. Reply with exactly one word."),
                Message::user("Say only: READY"),
            ],
            None,
        )
        .await;
    match ping {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Readiness check failed (LLM not reachable or bad config): {}", e);
            eprintln!("Fix the error above before using the agent.");
            std::process::exit(1);
        }
    }

    let llm_arc: Arc<Box<dyn memory_mcp::agent::AgentLLMProvider>> = Arc::new(llm);

    let agent = MemoryChatAgent::new(
        user_id.clone(),
        Arc::new(embedder),
        Arc::new(store),
        Arc::new(config.clone()),
        llm_arc.clone(),
    )
    .with_cached_index()
    .await?;

    let runtime = AgentRuntime::new_with_arc(llm_arc);

    eprintln!("Memory chat agent ready. User: {user_id}. Type a message and press Enter (empty to exit).");
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?.trim().to_string();
        if line.is_empty() {
            break;
        }
        match runtime.run(&agent, &line).await {
            Ok(out) => println!("{}\n", out),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}
