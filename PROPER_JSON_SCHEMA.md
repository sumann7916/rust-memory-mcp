# Proper JSON Schema with Required/Optional Fields

## Problem
The agent couldn't determine which parameters were required because the JSON schema wasn't clear enough about:
1. Which fields are truly required
2. Which fields are conditionally required based on scope
3. What the valid values are

## Solution - Enhanced Rust Schema Annotations

We added proper `#[schemars(...)]` annotations to the parameter structs to generate clear JSON schemas.

## SaveMemoryParams Schema

### Before
```rust
pub struct SaveMemoryParams {
    pub content: String,
    pub user_id: String,
    #[serde(default)]
    pub topics: Vec<String>,
    pub scope: String,
    // ... etc
}
```

Problems:
- No descriptions in generated JSON
- Hard to tell what's required vs optional
- No explanation of conditional requirements

### After
```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "SaveMemoryParams", description = "Parameters for saving a memory")]
pub struct SaveMemoryParams {
    #[schemars(description = "The memory content to save (REQUIRED)")]
    pub content: String,
    
    #[schemars(description = "User ID (REQUIRED)")]
    pub user_id: String,
    
    #[serde(default)]
    #[schemars(description = "Topics/tags for categorization (optional, defaults to empty array)")]
    pub topics: Vec<String>,
    
    #[schemars(
        description = "Scope of the memory (REQUIRED): 'global' (universal), 'lang' (language-specific), 'feature' (product feature), 'repo' (repository), or 'module' (directory)"
    )]
    pub scope: String,
    
    #[serde(default)]
    #[schemars(description = "Repository name (optional, but REQUIRED for 'repo' and 'module' scopes)")]
    pub repo: Option<String>,
    
    #[serde(default)]
    #[schemars(description = "Programming language (optional, but REQUIRED for 'lang' scope)")]
    pub lang: Option<String>,
    
    #[serde(default)]
    #[schemars(description = "Module/directory path like 'src/auth' (optional, but REQUIRED for 'module' scope)")]
    pub module: Option<String>,
    
    #[serde(default)]
    #[schemars(description = "Feature/product area name (optional, but REQUIRED for 'feature' scope)")]
    pub feature: Option<String>,
}
```

## Generated JSON Schema

The enhanced Rust annotations generate this JSON schema:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SaveMemoryParams",
  "description": "Parameters for saving a memory",
  "type": "object",
  "properties": {
    "content": {
      "description": "The memory content to save (REQUIRED)",
      "type": "string"
    },
    "user_id": {
      "description": "User ID (REQUIRED)",
      "type": "string"
    },
    "topics": {
      "description": "Topics/tags for categorization (optional, defaults to empty array)",
      "type": "array",
      "default": [],
      "items": {
        "type": "string"
      }
    },
    "scope": {
      "description": "Scope of the memory (REQUIRED): 'global' (universal), 'lang' (language-specific), 'feature' (product feature), 'repo' (repository), or 'module' (directory)",
      "type": "string"
    },
    "repo": {
      "description": "Repository name (optional, but REQUIRED for 'repo' and 'module' scopes)",
      "type": ["string", "null"],
      "default": null
    },
    "lang": {
      "description": "Programming language (optional, but REQUIRED for 'lang' scope)",
      "type": ["string", "null"],
      "default": null
    },
    "module": {
      "description": "Module/directory path like 'src/auth' (optional, but REQUIRED for 'module' scope)",
      "type": ["string", "null"],
      "default": null
    },
    "feature": {
      "description": "Feature/product area name (optional, but REQUIRED for 'feature' scope)",
      "type": ["string", "null"],
      "default": null
    }
  },
  "required": ["content", "user_id", "scope"]
}
```

## Key Schema Features

### 1. Required Fields (no default, not nullable)
```json
"scope": {
  "description": "Scope of the memory (REQUIRED): ...",
  "type": "string"  // ← Just "string", no null, no default
}
```

### 2. Optional Fields (default value)
```json
"topics": {
  "description": "Topics/tags (optional, defaults to empty array)",
  "type": "array",
  "default": []  // ← Has a default value
}
```

### 3. Conditionally Required Fields
```json
"repo": {
  "description": "Repository name (optional, but REQUIRED for 'repo' and 'module' scopes)",
  "type": ["string", "null"],  // ← Can be null
  "default": null               // ← Defaults to null
}
```

## How LLMs Read This Schema

When an LLM sees this schema, it understands:

1. **Always required**: Fields in `"required"` array with no default
   - `content`, `user_id`, `scope` ← **must always include**

2. **Optional with defaults**: Fields with `"default"` value
   - `topics` defaults to `[]` ← **can omit, will use empty array**

3. **Conditionally required**: Fields marked "optional, but REQUIRED for X"
   - `repo` is optional BUT required if scope="repo" or scope="module"
   - `lang` is optional BUT required if scope="lang"
   - `module` is optional BUT required if scope="module"
   - `feature` is optional BUT required if scope="feature"

## Testing the Schema

To see the generated schema:

```bash
cd mcp
cargo run --example test_schema
```

## Deployment

After changing the schema:

1. **Build the MCP server**:
   ```bash
   cd mcp
   cargo build --release
   ```

2. **Restart MCP server** (Cursor will do this automatically):
   - Cursor detects MCP server changes
   - Reconnects and fetches new tool schemas
   - Generates updated JSON files in `~/.cursor/projects/.../mcps/user-user-memory/tools/`

3. **Verify schema was updated**:
   ```bash
   cat ~/.cursor/projects/Users-sumankhadka-Portpro-rust-mem/mcps/user-user-memory/tools/save_memory.json
   ```
   
   Check that:
   - ✅ `"required"` array includes `["content", "user_id", "scope"]`
   - ✅ Property descriptions include "(REQUIRED)" or "(optional)" markers
   - ✅ Conditional requirements are explained in descriptions

## Why This Fixes The Issue

### Before
```
Agent sees: scope (string) with vague description
Agent thinks: "Maybe this is optional?"
Agent calls: save_memory without scope
Result: ❌ Error "missing field scope"
```

### After
```
Agent sees: 
  - scope in "required" array
  - type: "string" (not nullable)
  - description: "REQUIRED: 'global'|'lang'|'feature'|'repo'|'module'"
Agent thinks: "I MUST include scope and choose from these 5 values"
Agent calls: save_memory with scope="global"
Result: ✅ Success
```

## Files Modified

- `mcp/src/tools.rs` - Added `#[schemars(...)]` annotations to:
  - `SaveMemoryParams`
  - `SearchMemoryParams`

## Benefits

1. **Self-documenting**: Schema includes all necessary documentation
2. **Type-safe**: Rust compiler enforces the schema matches the struct
3. **LLM-friendly**: Clear descriptions with REQUIRED markers
4. **Maintainable**: Change the struct annotations, schema updates automatically
5. **Standard**: Uses JSON Schema standard that all tools understand

## For Future Parameter Structs

Always include:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "MyParams", description = "What this is for")]
pub struct MyParams {
    #[schemars(description = "Clear description (REQUIRED if not Option<>)")]
    pub required_field: String,
    
    #[serde(default)]
    #[schemars(description = "Clear description (optional, defaults to X)")]
    pub optional_with_default: Vec<String>,
    
    #[serde(default)]
    #[schemars(description = "Clear description (optional, but REQUIRED when Y)")]
    pub conditional: Option<String>,
}
```

Key rules:
- ✅ Use `#[schemars(description = "...")]` on every field
- ✅ Mark required fields with "(REQUIRED)" in description
- ✅ Explain defaults: "(optional, defaults to X)"
- ✅ Explain conditional requirements: "(optional, but REQUIRED when Y)"
- ✅ List valid enum values in the description
- ✅ Give examples in the description if complex
