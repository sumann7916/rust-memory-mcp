# Memory MCP Integration Rules

You have access to a memory system via the `user-user-memory` MCP server. Use it to remember and recall information across conversations.

## Available Tools

- `save_memory` - Save new memories
- `search_memory` - Search memories by semantic similarity
- `get_all_memories` - List all memories for a user
- `delete_memory` - Delete a specific memory

## When to Search Memory

**BEFORE any of these actions**, call `search_memory`:
- Responding to coding questions
- Generating or reviewing code
- Creating implementation plans
- Making architectural decisions
- Debugging or fixing issues
- Suggesting refactors

Call with:
- `query`: The user's question or topic
- `user_id`: "sumankhadka"
- `repo`: Current repository name (e.g., "rust-mem")
- `lang`: Current programming language if known

This returns both global preferences and repo-specific knowledge.

## When to Save Memory

Call `save_memory` when:
- User corrects you or shows a preference
- You solve a non-obvious problem
- User states a rule, convention, or pattern
- Something would be useful to remember next time
- A working solution is found after debugging

Parameters:
- `content`: The raw information to remember
- `user_id`: "sumankhadka"
- `repo`: Current repository name (optional - server auto-detects if global)
- `lang`: Programming language (optional)

The server automatically:
1. Extracts a clean, single-sentence fact
2. Classifies it (preference, business_logic, repo, general)
3. Decides if it's global or repo-specific

## Memory Categories

- **preference** - Coding habits, style choices, patterns that apply universally
- **business_logic** - Domain rules, product decisions, workflow rules
- **repo** - Knowledge specific to one codebase
- **general** - Everything else

## Example Usage

```
# Before answering a question about error handling
search_memory(query="error handling preferences", user_id="sumankhadka", repo="rust-mem", lang="rust")

# After user says "I prefer Result over panic"
save_memory(content="User prefers using Result type over panic for error handling", user_id="sumankhadka", lang="rust")
```

## Planning Mode

When creating implementation plans:
1. Search memory for relevant patterns, preferences, and past decisions
2. Apply remembered coding style preferences to the plan
3. Reference any repo-specific conventions or architecture decisions
4. After plan is approved and implemented, save key learnings

## Important Notes

- Global memories (category: preference) surface in ALL searches regardless of repo
- Always check memories before making assumptions about coding style
- Save memories proactively - don't wait for explicit requests
- The server handles fact extraction and classification automatically
- Memory search is fast - use it liberally to provide consistent, personalized responses
