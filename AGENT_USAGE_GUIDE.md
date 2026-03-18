# Memory Chat Agent - Quick Usage Guide

## Running the Improved Agent

```bash
cd /Users/sumankhadka/Portpro/rust-mem/mcp

# With Ollama (local)
LLM_PROVIDER=ollama \
EMBEDDER_PROVIDER=ollama \
AGENT_PROVIDER=ollama \
USER_ID=sumankhadka \
cargo run --release --bin memory_chat_agent

# With Gemini (cloud)
GEMINI_API_KEY=your_key \
LLM_PROVIDER=gemini \
EMBEDDER_PROVIDER=gemini \
AGENT_PROVIDER=gemini \
USER_ID=sumankhadka \
cargo run --release --bin memory_chat_agent
```

## New Features

### 1. Configure Search Settings

You can now dynamically adjust search parameters:

```
User: use min_score 0.2
Agent: [Calls configure_search(min_score=0.2)]
       Search configuration updated: min_score=0.2

User: show me 10 results
Agent: [Calls configure_search(limit=10)]
       Search configuration updated: limit=10

User: use min_score 0.3 and limit 8
Agent: [Calls configure_search(min_score=0.3, limit=8)]
       Search configuration updated: min_score=0.3, limit=8
```

### 2. Better Topic Matching

The agent now finds topics even with different formatting:

```
User: search for highway service
Agent: [Matches "highway-service" topic]
       [Returns memories with highway-service tag]

User: show me mongo stuff
Agent: [Matches "mongodb" topic via prefix matching]
       [Returns mongodb-related memories]

User: find auth jwt memories
Agent: [Matches "auth", "jwt", "authentication" topics]
       [Returns relevant authentication memories]
```

### 3. Persistent Settings

Settings persist across searches in the same session:

```
User: use min_score 0.2
Agent: Search configuration updated: min_score=0.2

User: search for mongodb
Agent: [Uses min_score=0.2 for search]

User: search for jwt
Agent: [Still uses min_score=0.2 - settings persisted!]
```

## Testing the Improvements

### Test Case 1: Original Problem from Terminal

```
User: give me list of my memory tags
Agent: [Shows topics including: highway-integration, highway-service, highway-sync]

User: now search memory related to highway-integration
Agent: [Finds memories with highway-integration topic]
       [Previously would return "no memories found"]
```

### Test Case 2: Dynamic min_score

```
User: use min_score 0.2
Agent: Search configuration updated: min_score=0.2

User: highway-service
Agent: [Returns results with scores as low as 0.2]
       [Previously always required 0.5 minimum]
```

### Test Case 3: Settings Persistence

```
User: configure min_score to 0.3
Agent: Search configuration updated: min_score=0.3

User: search mongodb
Agent: [Search uses min_score=0.3]

User: search jwt
Agent: [Still uses min_score=0.3 - not reset to 0.5]
```

## How It Works

1. **SearchConfig State:** Agent maintains thread-safe configuration state
2. **ConfigureSearchTool:** LLM calls this tool when user requests setting changes
3. **before_llm Hook:** Reads current settings before each search
4. **Enhanced Matching:** Normalizes and matches topics with multiple strategies

## Default Settings

- **min_score:** 0.5 (same as before)
- **limit:** 5 results (same as before)

Settings can be changed at any time during the session.

## System Prompt Additions

The agent now knows:
- How to use `configure_search` tool
- That settings persist across turns
- Current search configuration is shown in system prompt

---

**For detailed technical information, see:** `AGENT_IMPROVEMENTS.md`
