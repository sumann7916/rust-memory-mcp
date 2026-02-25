# Migration Fix for v1.x to v2.x

## Issue

When calling `get_memory_index`, you got error: "missing field 'scope'"

## Root Cause

You had existing memories in Qdrant from v1.x that used:
- `category: String` field (old)
- No `scope` field (new)
- No `topics` field (new)

When v2.x tried to deserialize those old memories, it failed because it expected the new schema.

## Fix Applied

Made `scope` field backward compatible:

```rust
// Before
pub scope: String,  // Required - would fail on old memories

// After  
#[serde(default = "default_scope")]
pub scope: String,  // Optional with default - handles old memories
```

**Default behavior:**
- Old memories without `scope` → defaults to `"global"`
- Old memories without `topics` → defaults to `[]` (empty array)

## What This Means

### ✅ Now Working
- `get_memory_index` will work with existing v1.x memories
- Old memories show up as scope: "global"
- Old memories show up with empty topics: []

### ⚠️ Limitations
- Old memories won't have accurate scope (all treated as global)
- Old memories won't have topics (can't benefit from topic overlap boost)
- Old memories still have old `category` field (ignored by v2.x)

## Recommendations

### Option 1: Keep Old Memories (Easier)
Just use the system as-is:
- Old memories work but all appear as "global" scope
- Gradually add new memories with proper scope + topics
- Over time, new memories will dominate

### Option 2: Clean Slate (Recommended)
Clear the collection and start fresh:

```bash
# Delete the collection
curl -X DELETE http://localhost:6333/collections/coding_memories

# Restart server - it will recreate collection
# Now all new memories will have proper scope + topics
```

### Option 3: Manual Migration (Advanced)
If you have important memories to preserve:

1. Export old memories:
   ```bash
   curl http://localhost:6333/collections/coding_memories/points/scroll > old_memories.json
   ```

2. Delete collection:
   ```bash
   curl -X DELETE http://localhost:6333/collections/coding_memories
   ```

3. Manually review each memory and re-save with proper scope + topics

## Testing After Fix

Now you should be able to call:

```javascript
get_memory_index({ user_id: "sumankhadka" })
```

Expected response:
```json
{
  "topics": [],  // Empty if all memories are old v1.x format
  "langs": [...],  // Any languages from old memories
  "repos": [...],  // Any repos from old memories
  "scopes": {
    "global": N  // All old memories counted as global
  },
  "total": N,
  "recent": [...]
}
```

## Next Steps

1. **Test**: Call `get_memory_index` again - should work now
2. **Decide**: Choose Option 1 (keep) or Option 2 (clean slate)
3. **Use**: Start saving new memories with proper scope + topics
4. **Restart Cursor**: Pick up the new binary (just rebuilt)

## Rebuild Status

✅ Binary rebuilt at: `/Users/sumankhadka/Portpro/rust-mem/mcp/target/release/memory-mcp`  
✅ Backward compatibility added for `scope` field  
✅ Old memories will default to `scope: "global"`, `topics: []`

---

**Recommendation**: I suggest **Option 2 (Clean Slate)** since you're just getting started with v2.x. This gives you a fresh start with proper scope + topics from the beginning.

To do this:
```bash
curl -X DELETE http://localhost:6333/collections/coding_memories
```

Then restart Cursor and the collection will be recreated with the new schema.
