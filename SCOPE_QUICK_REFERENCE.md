# Scope Parameter Quick Reference

## What is Scope?

`scope` determines how broadly a memory applies - from everywhere (global) to a specific file (module).

## The Decision Tree

Ask yourself: **"Where should this memory apply?"**

```
┌─────────────────────────────────────────────────────────────┐
│ Would I want this in EVERY project I ever work on?         │
│ ├─ YES → scope="global"                                     │
│ └─ NO ↓                                                      │
│                                                              │
│ Would I want this in every [LANGUAGE] project?              │
│ ├─ YES → scope="lang", lang="rust|python|js|..."           │
│ └─ NO ↓                                                      │
│                                                              │
│ Would I want this whenever working on [FEATURE/DOMAIN]?     │
│ ├─ YES → scope="feature", feature="auth|payments|..."      │
│ └─ NO ↓                                                      │
│                                                              │
│ Is this specific to [REPOSITORY]'s architecture?            │
│ ├─ YES → scope="repo", repo="project-name"                 │
│ └─ NO ↓                                                      │
│                                                              │
│ Is this specific to one file/directory?                     │
│ └─ YES → scope="module", repo="...", module="src/auth"     │
└─────────────────────────────────────────────────────────────┘
```

## Examples

### Global Scope
Universal preferences that apply everywhere.

```rust
save_memory(
    content="Prefer descriptive variable names over short abbreviations",
    user_id="sumankhadka",
    scope="global",
    topics=["code-style", "naming"]
)
```

### Language Scope
Language or technology-specific patterns.

```rust
save_memory(
    content="In Rust, prefer Result<T, E> over panic! for error handling",
    user_id="sumankhadka",
    scope="lang",
    lang="rust",
    topics=["error-handling", "rust"]
)
```

### Feature Scope
Product feature or domain logic across repos.

```rust
save_memory(
    content="Invoice generation uses Stripe API and requires tax calculation",
    user_id="sumankhadka",
    scope="feature",
    feature="invoicing",
    topics=["payments", "invoicing", "stripe"]
)
```

### Repository Scope
Codebase-specific architecture or patterns.

```rust
save_memory(
    content="Quote model has foreign key relationship to Vendor via vendor_id",
    user_id="sumankhadka",
    scope="repo",
    repo="rust-mem",
    topics=["database", "models", "relationships"]
)
```

### Module Scope
File or directory-specific implementation details.

```rust
save_memory(
    content="src/auth uses JWT with RS256 signing and 15-minute expiry",
    user_id="sumankhadka",
    scope="module",
    repo="my-api",
    module="src/auth",
    topics=["auth", "jwt", "security"]
)
```

## Required Parameters by Scope

| Scope    | Required Additional Params | Example Values                  |
|----------|---------------------------|---------------------------------|
| global   | none                      | -                               |
| lang     | `lang`                    | "rust", "python", "typescript"  |
| feature  | `feature`                 | "auth", "payments", "invoicing" |
| repo     | `repo`                    | "rust-mem", "my-api"           |
| module   | `repo`, `module`          | repo="my-api", module="src/auth"|

## Scope Boost Values (Search Ranking)

How much each scope boosts search results:

- `module`: **2.0x** - Highest boost (exact module match)
- `repo`: **1.6x** - Repo-specific memories
- `feature`: **1.4x** - Feature-specific memories
- `lang`: **1.3x** - Language-specific memories
- `global`: **1.0x** - Always included in search results
- Non-matching: **0.5x** - Penalty for irrelevant scope

Global memories **always surface** in searches. More specific scopes get bigger boosts when they match your current context.

## Tips

1. **When in doubt, go more specific**: 
   - `module` > `repo` > `feature` > `lang` > `global`
   - You can always broaden scope later

2. **Reuse topics**: 
   - Call `get_memory_index()` first to see existing topics
   - Prefer "auth" if it exists over creating "authentication"

3. **One memory, one fact**:
   - "JWT uses RS256 and expires in 15 min" ✅
   - Don't save entire code explanations ❌

4. **Save proactively**:
   - User shows preference → save it
   - You solve a non-obvious problem → save it
   - User corrects you → use `correct_memory()` instead

5. **Search before coding**:
   - Always call `search_memory()` before generating code
   - Pass current context (repo, lang, module) for boost ranking
