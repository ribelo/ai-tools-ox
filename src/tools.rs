use std::{collections::HashMap, fmt, sync::Arc};

use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::Jsonify;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    #[serde(rename = "type")]
    argument_type: String,
    description: String,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    argument_enum: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    #[derivative(Default(value = "String::from(\"object\")"))]
    parameter_type: String,
    properties: HashMap<String, ToolParameter>,
    required: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub function: ToolCallFunction,
}

#[derive(Debug, Error)]
pub enum ToolBuilderError {
    #[error("Name not set")]
    NameNotSet,
    #[error("Description not set")]
    DescriptionNotSet,
}

#[derive(Default)]
pub struct ToolBuilder {
    name: Option<String>,
    description: Option<String>,
    parameters: Option<ToolParameters>,
}

impl ToolBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    pub fn add_parameter<T: Jsonify>(
        mut self,
        name: impl ToString,
        description: impl ToString,
    ) -> Self {
        let argument = ToolParameter {
            argument_type: T::jsonify().as_str().unwrap().to_string(),
            description: description.to_string(),
            argument_enum: None,
        };
        let mut arguments = self.parameters.unwrap_or_default();
        arguments.properties.insert(name.to_string(), argument);
        arguments.required.push(name.to_string());
        self.parameters = Some(arguments);

        self
    }
    pub fn add_optional_parameter<T: Jsonify>(
        mut self,
        name: impl ToString,
        description: impl ToString,
    ) -> Self {
        let argument = ToolParameter {
            argument_type: T::jsonify().as_str().unwrap().to_string(),
            description: description.to_string(),
            argument_enum: None,
        };
        let mut arguments = self.parameters.unwrap_or_default();
        arguments.properties.insert(name.to_string(), argument);
        self.parameters = Some(arguments);

        self
    }
    pub fn add_enum_parameter(
        mut self,
        name: impl ToString,
        description: impl ToString,
        enum_values: impl IntoIterator<Item = impl ToString>,
    ) -> Self {
        let variants = enum_values
            .into_iter()
            .map(|value| value.to_string())
            .collect();
        let argument = ToolParameter {
            argument_type: "string".to_string(),
            description: description.to_string(),
            argument_enum: Some(variants),
        };
        let mut arguments = self.parameters.unwrap_or_default();
        arguments.properties.insert(name.to_string(), argument);
        arguments.required.push(name.to_string());
        self.parameters = Some(arguments);

        self
    }
    pub fn add_optional_enum_parameter(
        mut self,
        name: impl ToString,
        description: impl ToString,
        enum_values: impl IntoIterator<Item = impl ToString>,
    ) -> Self {
        let variants = enum_values
            .into_iter()
            .map(|value| value.to_string())
            .collect();
        let argument = ToolParameter {
            argument_type: "string".to_string(),
            description: description.to_string(),
            argument_enum: Some(variants),
        };
        let mut arguments = self.parameters.unwrap_or_default();
        arguments.properties.insert(name.to_string(), argument);
        self.parameters = Some(arguments);

        self
    }
    pub fn build(self) -> Result<Tool, ToolBuilderError> {
        let name = self.name.ok_or(ToolBuilderError::NameNotSet)?;
        let description = self
            .description
            .ok_or(ToolBuilderError::DescriptionNotSet)?;
        let parameters = self.parameters.unwrap_or_default();
        let function = ToolFunction {
            name,
            description,
            parameters,
        };

        Ok(Tool {
            tool_type: ToolType::Function,
            function,
        })
    }
}

#[async_trait::async_trait]
pub trait ToTool: fmt::Debug + Send + Sync {
    fn to_tool(&self) -> Tool;
    async fn call_tool(&self, id: &str, input: serde_json::Value) -> ToolCallResult;
}

#[derive(Debug, Clone, Default)]
pub struct Tools(pub HashMap<String, (serde_json::Value, Arc<dyn ToTool>)>);

impl Tools {
    pub fn add_tool<T>(mut self, toolable: T) -> Self
    where
        T: ToTool + 'static,
    {
        let tool = toolable.to_tool();
        let json = serde_json::to_value(&tool).unwrap();
        let name = tool.function.name.clone();
        self.0.insert(name, (json, Arc::new(toolable)));
        self
    }
    async fn call_tool(&self, tool_call: &ToolCall) -> ToolCallResult {
        let function_name = &tool_call.function.name;
        let id = &tool_call.id;
        if let Some((_, tool)) = self.0.get(function_name) {
            let json = serde_json::from_str(&tool_call.function.arguments).unwrap();
            tool.call_tool(id, json).await
        } else {
            ToolCallResult {
                tool_call_id: id.clone(),
                content: json!("Tool not found").to_string(),
            }
        }
    }
    #[must_use]
    pub async fn call_tools(&self, tool_calls: &[ToolCall]) -> ToolsResults {
        let mut results = ToolsResults::new();
        for tool_call in tool_calls {
            let result = self.call_tool(tool_call).await;
            results.add_result(result);
        }
        results
    }
}

impl serde::Serialize for Tools {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0
            .values()
            .map(|(json, _)| json)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_call_id: String,
    pub content: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ToolsResults(pub Vec<ToolCallResult>);

impl ToolsResults {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add_result(&mut self, result: ToolCallResult) {
        self.0.push(result);
    }
}
