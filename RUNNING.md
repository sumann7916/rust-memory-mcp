# Running the Memory MCP Server

## Status: ✅ Ready to Use

The Memory MCP server v2.1.0 has been built and is ready to run with the new `get_memory_index` tool.

## Prerequisites

- ✅ Qdrant running (confirmed at http://localhost:6334)
- ✅ Binary built at `mcp/target/release/memory-mcp`
- ✅ 6 tools available (including new `get_memory_index`)

## Quick Start

### Option 1: Use with Cursor (Recommended)

The server is already configured in your Cursor MCP settings. To use it:

1. **Restart Cursor** to pick up the new binary
2. **In Cursor chat**, the memory tools are automatically available

### Option 2: Direct Testing

If you want to test the server directly:

```bash
cd /Users/sumankhadka/Portpro/rust-mem/mcp
./target/release/memory-mcp
```

The server communicates via stdio using JSON-RPC protocol.

## Using the New get_memory_index Tool

### In Cursor Chat

You can now ask Cursor to use the memory system:

**Session Start (Recommended):**
```
At the start of this session, please call get_memory_index to see 
what topics already exist in my memory bank.
```

**Saving Memories:**
```
Remember that JWT refresh tokens should be rotated on every use.
(Cursor will automatically check existing topics and reuse them)
```

**Searching Memories:**
```
What do I know about JWT token handling?
(Cursor will search your memories with boost ranking)
```

### Manual MCP Call (Advanced)

If calling the MCP server directly:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "get_memory_index",
    "arguments": {
      "user_id": "sumankhadka"
    }
  }
}
```

## Available Tools

1. **get_memory_index** (NEW!) - Get existing topics for consistency
   - Call at session start
   - Returns: topics, langs, repos, scopes, total, recent

2. **save_memory** - Save new memories
   - LLM pre-processes: extract fact, determine topics, decide scope
   - Server: validates, deduplicates, stores

3. **search_memory** - Search with boost ranking
   - Context fields (repo, lang, module) used for boost
   - Returns ranked results with scores

4. **get_all_memories** - List all memories
   - Returns everything including superseded

5. **delete_memory** - Delete a specific memory
   - Requires memory_id

6. **correct_memory** - Correct an existing memory
   - Supersedes old, creates new with higher confidence

## Configuration

Current environment (using Ollama by default):

```bash
# Embedder (required)
EMBEDDER_PROVIDER=ollama           # or gemini, openai
EMBEDDER_MODEL=nomic-embed-text    # default for ollama

# Qdrant (required)
QDRANT_HOST=localhost
QDRANT_PORT=6334
QDRANT_COLLECTION=coding_memories

# Boost multipliers (optional)
SCOPE_BOOST_LANG=1.3      # default
SCOPE_BOOST_FEATURE=1.4   # default
SCOPE_BOOST_REPO=1.6      # default
SCOPE_BOOST_MODULE=2.0    # default
TOPIC_BOOST_MAX=0.3       # default
```

## Testing the New Feature

### Test 1: Get Memory Index

Ask Cursor:
```
Call get_memory_index for my user ID and show me what topics exist.
```

Expected response:
- List of existing topics (sorted)
- Languages in use
- Repositories with memories
- Scope distribution
- Recent memory summaries

### Test 2: Topic Consistency

Ask Cursor:
```
Save a memory: "JWT tokens should expire after 15 minutes"
```

Cursor should:
1. Check cached topics from get_memory_index
2. Find existing "auth", "jwt", "security" topics
3. Reuse those instead of creating new variants
4. Save with consistent topics

### Test 3: Search with Boost

Ask Cursor:
```
Search my memories about JWT authentication
```

Cursor should:
1. Perform semantic search
2. Apply boost multipliers based on scope
3. Return ranked results with scores

## Workflow Example

### Ideal Session Flow

```
1. Session starts
   → Cursor calls: get_memory_index(user_id="sumankhadka")
   → Caches: topics=["auth", "jwt", "security", "payments", ...]

2. You ask: "How should I handle JWT tokens?"
   → Cursor calls: search_memory(query="JWT token handling", ...)
   → Returns: memories ranked by semantic_score * boost

3. You say: "Remember to rotate refresh tokens"
   → Cursor checks cached topics → finds "auth", "jwt", "security"
   → Cursor calls: save_memory(content="...", topics=["auth", "jwt", "security"], ...)
   → Server: validates, deduplicates, stores

4. Later in session, you save another JWT memory
   → Cursor reuses same topics from cache
   → Topic consistency maintained!
```

## Verifying It Works

### Check Server Logs

If you want to see server activity, you can check Cursor's MCP logs:
- On Mac: `~/Library/Logs/Cursor/` 
- Look for memory-mcp related output

### Check Qdrant

Verify memories are being stored:

```bash
# Get collection info
curl http://localhost:6333/collections/coding_memories

# Count points
curl http://localhost:6333/collections/coding_memories/points/count
```

## Troubleshooting

### Server Not Starting

1. Check Qdrant is running:
   ```bash
   docker ps | grep qdrant
   ```

2. Check binary exists:
   ```bash
   ls -lh /Users/sumankhadka/Portpro/rust-mem/mcp/target/release/memory-mcp
   ```

3. Restart Cursor to reload MCP configuration

### Tools Not Available

1. Check Cursor MCP settings (`.cursor/mcp.json`)
2. Verify the binary path is correct
3. Restart Cursor completely

### Topic Consistency Not Working

1. Verify get_memory_index is being called at session start
2. Check that Cursor is caching the topic list
3. Confirm that Cursor checks cached topics before saving

## Next Steps

1. **Use it naturally in Cursor** - Just start a conversation and ask Cursor to remember things
2. **Monitor topic consistency** - Over time, you should see consistent topic usage
3. **Check memory index** - Periodically ask to see what's in your memory bank
4. **Provide feedback** - Note any issues with topic selection or boost ranking

## Documentation

- **README.md** - Full architecture and API documentation
- **MEMORY_RULES.md** - Integration guide for LLMs
- **MEMORY_PROMPT.md** - Quick reference snippets
- **GET_MEMORY_INDEX_ENHANCEMENT.md** - Details on the new feature

---

**Version:** v2.1.0  
**Status:** ✅ Ready to use  
**Build:** Successful (release mode)  
**Server:** Running (managed by Cursor)
