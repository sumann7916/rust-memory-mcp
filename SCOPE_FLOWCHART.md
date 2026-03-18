# Scope Selection Flowchart

```
START: Need to save a memory
    |
    v
┌─────────────────────────────────────────────────┐
│ Is this a universal preference or pattern      │
│ that applies to ALL projects?                   │
│                                                  │
│ Examples:                                        │
│ - "Prefer descriptive variable names"           │
│ - "Always add error handling"                   │
│ - "Document public APIs"                        │
└─────────────────────────────────────────────────┘
    |
    ├─ YES → scope = "global"
    |        (no additional params needed)
    |
    └─ NO
       |
       v
┌─────────────────────────────────────────────────┐
│ Is this specific to a PROGRAMMING LANGUAGE      │
│ or technology stack?                             │
│                                                  │
│ Examples:                                        │
│ - "In Rust, prefer Result over panic"           │
│ - "Use async/await in JavaScript"               │
│ - "Prefer dataclasses in Python"                │
└─────────────────────────────────────────────────┘
    |
    ├─ YES → scope = "lang"
    |        (requires: lang="rust|python|...")
    |
    └─ NO
       |
       v
┌─────────────────────────────────────────────────┐
│ Is this about a PRODUCT FEATURE or domain       │
│ that spans multiple repositories?               │
│                                                  │
│ Examples:                                        │
│ - "Invoicing uses Stripe API"                   │
│ - "Auth requires 2FA for admin users"           │
│ - "File uploads go to S3"                       │
└─────────────────────────────────────────────────┘
    |
    ├─ YES → scope = "feature"
    |        (requires: feature="auth|payments|...")
    |
    └─ NO
       |
       v
┌─────────────────────────────────────────────────┐
│ Is this specific to one REPOSITORY's            │
│ architecture or data model?                     │
│                                                  │
│ Examples:                                        │
│ - "Quote has FK to Vendor"                      │
│ - "This repo uses event sourcing"               │
│ - "API uses GraphQL not REST"                   │
└─────────────────────────────────────────────────┘
    |
    ├─ YES → scope = "repo"
    |        (requires: repo="project-name")
    |
    └─ NO
       |
       v
┌─────────────────────────────────────────────────┐
│ Is this specific to one MODULE/FILE/DIRECTORY?  │
│                                                  │
│ Examples:                                        │
│ - "src/auth uses JWT with RS256"                │
│ - "api/handlers validates input with serde"     │
│ - "db/migrations uses sqlx"                     │
└─────────────────────────────────────────────────┘
    |
    └─ YES → scope = "module"
             (requires: repo="...", module="src/auth")

═══════════════════════════════════════════════════

RULE OF THUMB: When in doubt, go more specific!

Specificity ranking (most to least):
module > repo > feature > lang > global

You can always broaden a memory later by correcting it.
It's harder to narrow down a memory that's too broad.

═══════════════════════════════════════════════════

SCOPE BOOST IN SEARCHES:

When searching, memories are ranked by:
  semantic_similarity × scope_boost

Scope boosts:
┌────────────┬───────┬─────────────────────────────┐
│ Scope      │ Boost │ When                        │
├────────────┼───────┼─────────────────────────────┤
│ module     │ 2.0x  │ Exact module match          │
│ repo       │ 1.6x  │ Same repository             │
│ feature    │ 1.4x  │ Same feature/domain         │
│ lang       │ 1.3x  │ Same language               │
│ global     │ 1.0x  │ Always (universal)          │
│ no match   │ 0.5x  │ Different scope context     │
└────────────┴───────┴─────────────────────────────┘

This means:
- Global memories ALWAYS appear in results
- More specific matches get prioritized
- Cross-repo knowledge still accessible (with penalty)

═══════════════════════════════════════════════════
```

## Examples with Decision Path

### Example 1: "I prefer Result over panic in Rust"

```
START
 ↓
Universal for all projects? NO
 ↓
Language-specific? YES
 ↓
RESULT: scope="lang", lang="rust"
```

### Example 2: "Quote model has FK to Vendor in rust-mem"

```
START
 ↓
Universal? NO
 ↓
Language-specific? NO (it's about data model, not language)
 ↓
Feature/domain? NO (it's specific to rust-mem's schema)
 ↓
Repository-specific? YES
 ↓
RESULT: scope="repo", repo="rust-mem"
```

### Example 3: "src/auth uses JWT with 15-min expiry"

```
START
 ↓
Universal? NO
 ↓
Language-specific? NO (it's about a specific module)
 ↓
Feature/domain? NO (implementation detail, not business logic)
 ↓
Repository-specific? NO (too specific to one directory)
 ↓
Module-specific? YES
 ↓
RESULT: scope="module", repo="my-api", module="src/auth"
```

### Example 4: "Always validate user input"

```
START
 ↓
Universal? YES (applies to every project)
 ↓
RESULT: scope="global"
```

### Example 5: "Invoicing uses Stripe for payment processing"

```
START
 ↓
Universal? NO
 ↓
Language-specific? NO (business logic, not code pattern)
 ↓
Feature/domain? YES (invoicing feature could be in multiple repos)
 ↓
RESULT: scope="feature", feature="invoicing"
```
