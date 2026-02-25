# Memory MCP Quick Prompt

Copy and paste this at the start of conversations to activate memory:

---

## Standard Version (Recommended)

```
🧠 Memory Active: Before we start, call get_memory_index to see existing topics. When saving memories, check cached topics first and reuse them. Search memories before coding tasks.
```

---

## Detailed Version

```
🧠 Memory System Active:
1. Start: Call get_memory_index(user_id="sumankhadka") and cache topics
2. Before coding: Search memories for relevant patterns
3. After solving: Save learnings (check cached topics, reuse when applicable)
4. User corrects: Use correct_memory not save_memory
```

---

## Minimal Version

```
🧠 Memory: get_memory_index → search before coding → save learnings (reuse cached topics)
```

---

## Ultra-Short Version

```
🧠 Use memory MCP (index→search→save with topic reuse)
```

---

## For Specific Tasks

### For Coding Session
```
🧠 Memory: Check my memory index first, then search for patterns before we start coding.
```

### For Saving Knowledge
```
🧠 Memory: I want you to remember this. Check existing topics first and reuse them.
```

### For Searching
```
🧠 Memory: Search my memories about [topic] before answering.
```

---

## Best Practice Reminder

Add to your Cursor settings or project README:

```markdown
## Memory System Usage

Prepend conversations with:
🧠 Memory: get_memory_index → search → save (reuse topics)

This activates the memory MCP server with:
- 6 tools: get_memory_index, save_memory, search_memory, get_all_memories, delete_memory, correct_memory
- Boost ranking: module(2.0x) > repo(1.6x) > feature(1.4x) > lang(1.3x) > global(1.0x)
- Topic consistency via cached index
```

---

## Recommendation

Use the **Standard Version** for most conversations:

```
🧠 Memory Active: Before we start, call get_memory_index to see existing topics. When saving memories, check cached topics first and reuse them. Search memories before coding tasks.
```

It's:
- Short enough to not waste tokens
- Clear about the workflow
- Reminds to use get_memory_index
- Emphasizes topic reuse
- Covers the main use cases
