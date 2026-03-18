# Memory System Troubleshooting

## Issue: Search Returns Empty Results Despite Having Memories

### Symptoms
- `get_all_memories` shows 50+ memories with topics and content
- `search_memory` returns empty `results: []` array
- Agent sees "tags but no memory content"

### Root Cause
The memory server's semantic search has a **default minimum score threshold** that's too high (estimated ~0.5-0.7). Most memories score between 0.43-0.50 on semantic similarity, which gets filtered out.

### Solution
Always include `min_score: 0.3` parameter in all `search_memory` calls:

```javascript
// ❌ BAD - Uses default threshold, returns nothing
search_memory({
  query: "mongodb performance",
  user_id: "sumankhadka",
  repo: "portpro-backend"
})

// ✅ GOOD - Explicit threshold, returns results
search_memory({
  query: "mongodb performance",
  user_id: "sumankhadka",
  repo: "portpro-backend",
  min_score: 0.3  // REQUIRED
})
```

### Score Interpretation
- **0.0-0.3**: Barely relevant, noise
- **0.3-0.4**: Somewhat relevant, useful context
- **0.4-0.5**: Relevant matches (most of your memories)
- **0.5-0.7**: Highly relevant
- **0.7-1.0**: Exact or near-exact matches

### Best Practices

1. **Always set min_score**: Include `min_score: 0.3` in every search
2. **Adjust if needed**: If no results, try `min_score: 0.2`
3. **Use limit wisely**: Set `limit: 10` for comprehensive context
4. **Check scores**: Review the `score` and `boost` values in results
5. **Fallback to get_all**: If search fails completely, use `get_all_memories` to verify data exists

### Updated Rule
The `.cursor/rules/memory.md` file has been updated to include `min_score: 0.3` in all examples and documentation.

---

## Testing Your Fix

```bash
# Test search WITHOUT min_score (will return nothing)
curl -X POST your-memory-server/search \
  -d '{"query": "mongodb", "user_id": "sumankhadka"}'

# Test search WITH min_score (will return results)
curl -X POST your-memory-server/search \
  -d '{"query": "mongodb", "user_id": "sumankhadka", "min_score": 0.3}'
```

## Future Improvements

1. **Server-side**: Ask memory server maintainer to lower default threshold to 0.3
2. **Embeddings**: Consider retraining embeddings if scores remain consistently low
3. **Memory quality**: Review and consolidate low-confidence memories
