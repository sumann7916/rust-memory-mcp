#!/usr/bin/env python3
import json
import os
import sys
from mem0 import Memory


def get_llm_config():
    provider = os.environ.get("LLM_PROVIDER", "ollama").lower()
    model = os.environ.get("LLM_MODEL")
    
    if provider == "ollama":
        return {
            "provider": "ollama",
            "config": {
                "model": model or "qwen2.5-coder:7b",
                "ollama_base_url": os.environ.get("OLLAMA_BASE_URL", "http://localhost:11434"),
            },
        }
    
    if provider == "gemini":
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            raise ValueError("GEMINI_API_KEY environment variable required for Gemini provider")
        return {
            "provider": "gemini",
            "config": {
                "model": model or "gemini-2.0-flash",
                "api_key": api_key,
            },
        }
    
    if provider == "openai":
        api_key = os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable required for OpenAI provider")
        return {
            "provider": "openai",
            "config": {
                "model": model or "gpt-4o-mini",
                "api_key": api_key,
            },
        }
    
    if provider == "anthropic":
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise ValueError("ANTHROPIC_API_KEY environment variable required for Anthropic provider")
        return {
            "provider": "anthropic",
            "config": {
                "model": model or "claude-3-haiku-20240307",
                "api_key": api_key,
            },
        }
    
    raise ValueError(f"Unknown LLM provider: {provider}")


def get_embedder_config():
    provider = os.environ.get("EMBEDDER_PROVIDER", "ollama").lower()
    model = os.environ.get("EMBEDDER_MODEL")
    
    if provider == "ollama":
        return {
            "provider": "ollama",
            "config": {
                "model": model or "nomic-embed-text",
                "ollama_base_url": os.environ.get("OLLAMA_BASE_URL", "http://localhost:11434"),
            },
        }
    
    if provider == "gemini":
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            raise ValueError("GEMINI_API_KEY environment variable required for Gemini embedder")
        return {
            "provider": "gemini",
            "config": {
                "model": model or "models/text-embedding-004",
                "api_key": api_key,
            },
        }
    
    if provider == "openai":
        api_key = os.environ.get("OPENAI_API_KEY")
        if not api_key:
            raise ValueError("OPENAI_API_KEY environment variable required for OpenAI embedder")
        return {
            "provider": "openai",
            "config": {
                "model": model or "text-embedding-3-small",
                "api_key": api_key,
            },
        }
    
    raise ValueError(f"Unknown embedder provider: {provider}")


def get_vector_store_config():
    chroma_host = os.environ.get("CHROMA_HOST")
    chroma_port = os.environ.get("CHROMA_PORT", "8000")
    
    if chroma_host:
        return {
            "provider": "chroma",
            "config": {
                "collection_name": os.environ.get("CHROMA_COLLECTION", "coding_memories"),
                "host": chroma_host,
                "port": int(chroma_port),
            },
        }
    
    script_dir = os.path.dirname(os.path.abspath(__file__))
    chroma_path = os.path.join(script_dir, "chroma_db")
    return {
        "provider": "chroma",
        "config": {
            "collection_name": os.environ.get("CHROMA_COLLECTION", "coding_memories"),
            "path": chroma_path,
        },
    }


def get_memory():
    config = {
        "llm": get_llm_config(),
        "embedder": get_embedder_config(),
        "vector_store": get_vector_store_config(),
    }
    
    return Memory.from_config(config)


def save_memory(payload):
    m = get_memory()
    content = payload["content"]
    user_id = payload["user_id"]
    tags = payload.get("tags", [])
    
    # System logic to determine if 'global' tag should be added
    should_add_global = False
    
    # Add 'global' tag if:
    # 1. No repo-specific tags (no 'repo:' prefix)
    # 2. No module-specific tags (no 'module:' prefix)
    # 3. Has category:style, category:preference, or no language-specific tags
    has_repo_tag = any(tag.startswith("repo:") for tag in tags)
    has_module_tag = any(tag.startswith("module:") for tag in tags)
    has_lang_tag = any(tag.startswith("lang:") for tag in tags)
    has_style_or_preference = any(tag in ["category:style", "category:preference"] for tag in tags)
    
    # If it's a style/preference with no specific context, it's likely global
    if has_style_or_preference and not has_repo_tag and not has_module_tag:
        should_add_global = True
    
    # If it has no specific context at all, consider it global
    if not has_repo_tag and not has_module_tag and not has_lang_tag:
        should_add_global = True
    
    # Add global tag if determined by system
    if should_add_global and "global" not in tags:
        tags.append("global")
    
    metadata = {"tags": tags} if tags else None
    
    result = m.add(
        content, 
        user_id=user_id, 
        agent_id="coding-agent",
        metadata=metadata
    )
    
    memories = []
    if result and "results" in result:
        for item in result["results"]:
            memory_item = {
                "id": item.get("id", ""),
                "memory": item.get("memory", ""),
                "tags": tags,
            }
            memories.append(memory_item)
    
    return {
        "success": True,
        "message": f"Memory saved for user {user_id}",
        "memories": memories,
    }


def search_memory(payload):
    m = get_memory()
    query = payload["query"]
    user_id = payload["user_id"]
    tags = payload.get("tags", [])
    limit = payload.get("limit", 5)
    
    # For now, search without filters and post-filter in Python
    # ChromaDB metadata filtering is limited for array fields
    results = m.search(
        query, 
        user_id=user_id, 
        agent_id="coding-agent", 
        limit=limit * 2  # Get more results to account for filtering
    )
    
    items = []
    if results and "results" in results:
        for item in results["results"]:
            metadata = item.get("metadata", {})
            item_tags = metadata.get("tags", []) if metadata else []
            
            # If tags specified, filter to only memories with matching tags
            if tags:
                if not any(tag in item_tags for tag in tags):
                    continue
            
            items.append({
                "id": item.get("id", ""),
                "memory": item.get("memory", ""),
                "tags": item_tags,
                "score": item.get("score"),
            })
            
            # Limit results after filtering
            if len(items) >= limit:
                break
    
    return {"results": items}


def get_all_memories(payload):
    m = get_memory()
    user_id = payload["user_id"]
    
    results = m.get_all(user_id=user_id, agent_id="coding-agent")
    
    memories = []
    if results and "results" in results:
        for item in results["results"]:
            metadata = item.get("metadata", {})
            item_tags = metadata.get("tags", []) if metadata else []
            memories.append({
                "id": item.get("id", ""),
                "memory": item.get("memory", ""),
                "tags": item_tags,
            })
    
    return {"memories": memories}


def delete_memory(payload):
    m = get_memory()
    memory_id = payload["memory_id"]
    
    m.delete(memory_id=memory_id)
    
    return {
        "success": True,
        "message": f"Memory {memory_id} deleted",
    }


def get_tags(payload):
    """Get all unique tags from stored memories."""
    m = get_memory()
    user_id = payload["user_id"]
    
    results = m.get_all(user_id=user_id, agent_id="coding-agent")
    
    # Extract all unique tags
    tags_set = set()
    if results and "results" in results:
        for item in results["results"]:
            metadata = item.get("metadata", {})
            item_tags = metadata.get("tags", []) if metadata else []
            tags_set.update(item_tags)
    
    return {"tags": sorted(list(tags_set))}


def main():
    if len(sys.argv) != 3:
        print(json.dumps({"error": "Usage: memory.py <operation> <json_payload>"}))
        sys.exit(1)
    
    operation = sys.argv[1]
    payload = json.loads(sys.argv[2])
    
    operations = {
        "save": save_memory,
        "search": search_memory,
        "get_all": get_all_memories,
        "delete": delete_memory,
        "get_tags": get_tags,
    }
    
    if operation not in operations:
        print(json.dumps({"error": f"Unknown operation: {operation}"}))
        sys.exit(1)
    
    result = operations[operation](payload)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
