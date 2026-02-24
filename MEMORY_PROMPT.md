# Quick Memory Prompt Snippet

Copy and paste this at the top of your prompts to remind the AI about memory:

---

**Memory System Reminder:**
1. Start by calling `get_tags` to see available contexts
2. Before coding, call `search_memory` with relevant tags (always include `global`)
3. Automatically save learnings with `save_memory` when solutions work or patterns are learned

Available tag types: `global`, `repo:NAME`, `lang:LANGUAGE`, `category:TYPE` (style/pattern/bug_fix/preference)

---

## Even Shorter Version:

```
🧠 Use memory: get_tags → search before coding → auto-save when things work
```

## Example Usage in Prompts:

### For any coding task:
```
🧠 Memory: Check tags, search patterns, auto-save learnings

[Your actual request here...]
```

### Quick reminder format:
```
🧠 Use memory system (search first, auto-save learnings)

[Your request...]
```
