use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::message::{LLMResponse, Message};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    async fn call(&self, args: Value) -> Result<Value>;
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn system_prompt(&self) -> String;
    fn tools(&self) -> Vec<Box<dyn Tool>>;

    async fn before_run(&self, input: &str) -> Result<String> {
        Ok(input.to_string())
    }

    async fn before_llm(&self, _messages: &mut Vec<Message>) -> Result<()> {
        Ok(())
    }

    async fn before_tool(&self, _tool: &str, _args: &mut Value) -> Result<()> {
        Ok(())
    }

    async fn after_llm(&self, _response: &mut LLMResponse) -> Result<()> {
        Ok(())
    }

    async fn after_tool(&self, _tool: &str, _result: &mut Value) -> Result<()> {
        Ok(())
    }

    async fn after_run(&self, _input: &str, _output: &str) -> Result<()> {
        Ok(())
    }
}
