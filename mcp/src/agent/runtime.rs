use std::sync::Arc;

use anyhow::{anyhow, Result};

use super::llm::AgentLLMProvider;
use super::message::{LLMResponse, Message, ToolDef};
use super::traits::{Agent, Tool};

pub struct AgentRuntime {
    llm: Arc<Box<dyn AgentLLMProvider>>,
}

impl AgentRuntime {
    pub fn new(llm: Box<dyn AgentLLMProvider>) -> Self {
        Self { llm: Arc::new(llm) }
    }

    pub fn new_with_arc(llm: Arc<Box<dyn AgentLLMProvider>>) -> Self {
        Self { llm }
    }

    pub async fn run(&self, agent: &dyn Agent, input: &str) -> Result<String> {
        let input = agent.before_run(input).await?;

        let mut messages = vec![
            Message::system(agent.system_prompt()),
            Message::user(&input),
        ];

        let tool_defs = build_tool_defs(&agent.tools());

        loop {
            agent.before_llm(&mut messages).await?;

            let tools_ref = if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs.as_slice())
            };

            let mut response = self.llm.chat(messages.clone(), tools_ref).await?;
            agent.after_llm(&mut response).await?;

            match response {
                LLMResponse::Text(text) => {
                    agent.after_run(&input, &text).await?;
                    return Ok(text);
                }
                LLMResponse::ToolCall {
                    id,
                    name,
                    mut arguments,
                } => {
                    agent.before_tool(&name, &mut arguments).await?;

                    let tools = agent.tools();
                    let tool = find_tool(&tools, &name)?;
                    let mut result = tool.call(arguments.clone()).await?;

                    agent.after_tool(&name, &mut result).await?;

                    messages.push(Message::assistant_tool_call(&id, &name, arguments));
                    messages.push(Message::tool_result(&id, &name, result));
                }
            }
        }
    }
}

fn build_tool_defs(tools: &[Box<dyn Tool>]) -> Vec<ToolDef> {
    tools
        .iter()
        .map(|t| ToolDef {
            name: t.name().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters(),
        })
        .collect()
}

fn find_tool<'a>(tools: &'a [Box<dyn Tool>], name: &str) -> Result<&'a Box<dyn Tool>> {
    tools
        .iter()
        .find(|t| t.name() == name)
        .ok_or_else(|| anyhow!("Tool not found: {}", name))
}
