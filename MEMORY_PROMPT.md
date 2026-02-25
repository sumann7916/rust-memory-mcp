# Quick Memory Prompt Snippet

Copy and paste this at the top of your prompts to remind the AI about memory:

---

**Memory System Reminder:**
1. Session start: `get_memory_index` → cache topics for consistency
2. Before coding: `search_memory` with query + context for boost
3. After solving: Check cached topics → reuse when applicable → `save_memory`
4. User corrects: `correct_memory` (not save_memory)
5. Pre-processing required: Extract clean fact, check cached topics, reuse existing when applicable, determine 2-5 topics, decide scope (global/lang/feature/repo/module)

---

## Even Shorter Version

```
🧠 Memory: session start (get_memory_index) → search → check cached topics → save (fact, topics, scope) → correct for fixes
```

## Example Usage in Prompts

### For any coding task:

```
🧠 Memory: search for patterns → pre-process learnings → save with scope

[Your actual request here...]
```

### Quick reminder format:

```
🧠 Use memory (start: get_memory_index, search context first, check cached topics, save pre-processed learnings, correct_memory for fixes)

[Your request...]
```

## Full Feature Reminder

```
🧠 Memory tools available:
- get_memory_index: Get existing topics at session start (cache for consistency)
- search_memory: Check patterns before coding (semantic + boost ranking)
- save_memory: YOU pre-process first (extract fact, check cached topics, reuse when applicable, determine topics, decide scope)
- correct_memory: Fix incorrect memories (supersedes old)
- Scope tiers: global (1.0x) → lang (1.3x) → feature (1.4x) → repo (1.6x) → module (2.0x)
```

## Pre-processing Checklist

When calling save_memory, YOU must:

```
✓ Extract clean fact (one clear sentence)
✓ Get memory index at session start (cache topics)
✓ Check cached topics for existing ones
✓ Reuse existing topics when applicable (consistency!)
✓ Determine 2-5 topics from conversation context (create new only if needed)
✓ Decide scope using decision tree:
  - Everywhere? → global
  - All [lang] projects? → lang
  - All [feature] work? → feature
  - This repo only? → repo
  - This file/dir only? → module
✓ Pass context (repo, lang, module, feature) for scoping
```

## Scope Decision Tree

```
Would I want this in EVERY project I ever work on?           → global
Would I want this in every [lang] project?                   → lang
Would I want this whenever working on [feature/domain]?      → feature
Is this specific to this one repo's architecture/decisions?  → repo
Is this specific to this one file/folder?                    → module
```

## Example Workflow

```
Session start:
→ get_memory_index(user_id="sumankhadka")
→ Cache: topics=["auth", "jwt", "security", "payments", "async"]

User: "JWT refresh tokens should be rotated on every use"

Your pre-processing:
→ Fact: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks"
→ Check cached topics: "auth", "jwt", "security" exist → reuse them!
→ Topics: ["auth", "jwt", "security"] (no need for "authentication" or "json-web-tokens")
→ Scope: "feature" (applies to auth across projects)

save_memory({
  content: "JWT refresh tokens should be rotated on every use and old token invalidated to prevent replay attacks",
  user_id: "sumankhadka",
  topics: ["auth", "jwt", "security"],
  scope: "feature",
  feature: "auth"
})
```
