# MCP Memory Server - Agent Instructions Updated

## ✅ What Was Updated

### 1. **INSTRUCTIONS.md** - Main Server Description
Location: `/Users/sumankhadka/.cursor/projects/Users-sumankhadka-Portpro-rust-mem/mcps/user-user-memory/INSTRUCTIONS.md`

**Added:**
- Clear architecture explanation (LLM vs Server responsibilities)
- **Critical section** emphasizing `min_score: 0.2` requirement
- Side-by-side wrong/correct examples
- Comprehensive "When to Search" and "When to Save" guidance
- Score interpretation guide (0.0-1.0 scale)
- Boost multiplier explanation

### 2. **search_memory.json** - Tool Description
Location: `/Users/sumankhadka/.cursor/projects/Users-sumankhadka-Portpro-rust-mem/mcps/user-user-memory/tools/search_memory.json`

**Updated description to:**
- Start with "CRITICAL: Always include min_score: 0.2"
- Explain why (default threshold returns empty results)
- Mark min_score as "REQUIRED" in parameter list
- Include user_id default: 'sumankhadka'

---

## 📋 What Agents Will See

When agents load the memory MCP server, they'll see:

```markdown
# Memory MCP Server

CRITICAL: Always include `min_score: 0.2` in all `search_memory` calls.

❌ WRONG - Returns empty results
search_memory({ query: "...", user_id: "sumankhadka" })

✅ CORRECT - Returns relevant memories
search_memory({ 
  query: "...", 
  user_id: "sumankhadka",
  min_score: 0.2,  // REQUIRED
  repo: "current-repo",
  lang: "javascript",
  limit: 10
})
```

---

## 🎯 Impact

**Before:** Agents read tool schema, didn't know about min_score issue → empty searches  
**After:** Agents see CRITICAL warning first → always include min_score: 0.2 → successful searches

---

## 📍 Files Updated

1. ✅ `~/.cursor/rules/memory.md` - User rule (min_score: 0.2)
2. ✅ `mcps/user-user-memory/INSTRUCTIONS.md` - Server instructions (comprehensive guide)
3. ✅ `mcps/user-user-memory/tools/search_memory.json` - Tool description (CRITICAL warning)
4. ✅ Memory system - Saved this update as a memory

---

## 🔄 How It Works

The MCP server's `INSTRUCTIONS.md` appears as **server use instructions** in the agent's context:

```xml
<mcp_file_system_server 
  name="user-user-memory" 
  serverUseInstructions="Memory server with LLM-first architecture...">
```

Agents read this **before** using any tools, so they'll see the min_score requirement immediately!

---

## ✅ Complete

Your memory system is now fully configured with clear agent instructions at the MCP level! 🚀
