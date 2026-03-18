#!/bin/bash

# Memory Chat Agent - Easy Startup Script
# This script sets all the necessary environment variables and starts the agent

cd "$(dirname "$0")"

export LLM_PROVIDER=ollama
export EMBEDDER_PROVIDER=ollama
export AGENT_PROVIDER=ollama
export USER_ID=sumankhadka

echo "🚀 Starting Memory Chat Agent..."
echo "   User: $USER_ID"
echo "   LLM Provider: $LLM_PROVIDER"
echo "   Embedder: $EMBEDDER_PROVIDER"
echo ""
echo "💡 Tip: Try these commands:"
echo "   - use min_score 0.2"
echo "   - tell me about [topic]"
echo "   - what memories do you have"
echo ""

cargo run --release --bin memory_chat_agent
