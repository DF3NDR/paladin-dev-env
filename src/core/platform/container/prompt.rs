use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;
use std::hash::{Hash, Hasher};
use std::collections::BTreeMap;
use thiserror::Error;
use crate::core::base::entity::node::Node;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PromptItem {
    pub node: Node<PromptData>,
}

impl Hash for PromptItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PromptData {
    pub prompt_type: PromptType,
    pub content_attachments: Vec<Uuid>, // References to ContentItems
    pub parameters: PromptParameters,
    pub context: Option<String>,
    pub expected_output: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PromptType {
    Text(TextPrompt),
    System(SystemPrompt),
    User(UserPrompt),
    Assistant(AssistantPrompt),
    Function(FunctionPrompt),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptParameters {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
}

impl PartialEq for PromptParameters {
    fn eq(&self, other: &Self) -> bool {
        self.max_tokens == other.max_tokens
            && self.temperature == other.temperature
            && self.top_p == other.top_p
            && self.frequency_penalty == other.frequency_penalty
            && self.presence_penalty == other.presence_penalty
            && self.stop_sequences == other.stop_sequences
    }
}

impl Eq for PromptParameters {}

impl PartialOrd for PromptParameters {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // First compare max_tokens
        match self.max_tokens.cmp(&other.max_tokens) {
            std::cmp::Ordering::Equal => {},
            ord => return Some(ord),
        }
        
        // Then compare temperature using bits representation for consistency
        match (self.temperature, other.temperature) {
            (Some(a), Some(b)) => {
                match a.to_bits().cmp(&b.to_bits()) {
                    std::cmp::Ordering::Equal => {},
                    ord => return Some(ord),
                }
            },
            (None, None) => {},
            (Some(_), None) => return Some(std::cmp::Ordering::Greater),
            (None, Some(_)) => return Some(std::cmp::Ordering::Less),
        }
        
        // Compare top_p
        match (self.top_p, other.top_p) {
            (Some(a), Some(b)) => {
                match a.to_bits().cmp(&b.to_bits()) {
                    std::cmp::Ordering::Equal => {},
                    ord => return Some(ord),
                }
            },
            (None, None) => {},
            (Some(_), None) => return Some(std::cmp::Ordering::Greater),
            (None, Some(_)) => return Some(std::cmp::Ordering::Less),
        }
        
        // Compare frequency_penalty
        match (self.frequency_penalty, other.frequency_penalty) {
            (Some(a), Some(b)) => {
                match a.to_bits().cmp(&b.to_bits()) {
                    std::cmp::Ordering::Equal => {},
                    ord => return Some(ord),
                }
            },
            (None, None) => {},
            (Some(_), None) => return Some(std::cmp::Ordering::Greater),
            (None, Some(_)) => return Some(std::cmp::Ordering::Less),
        }
        
        // Compare presence_penalty
        match (self.presence_penalty, other.presence_penalty) {
            (Some(a), Some(b)) => {
                match a.to_bits().cmp(&b.to_bits()) {
                    std::cmp::Ordering::Equal => {},
                    ord => return Some(ord),
                }
            },
            (None, None) => {},
            (Some(_), None) => return Some(std::cmp::Ordering::Greater),
            (None, Some(_)) => return Some(std::cmp::Ordering::Less),
        }
        
        // Finally compare stop_sequences
        Some(self.stop_sequences.cmp(&other.stop_sequences))
    }
}

impl Ord for PromptParameters {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Hash for PromptParameters {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.max_tokens.hash(state);
        if let Some(temp) = self.temperature {
            temp.to_bits().hash(state);
        }
        if let Some(top_p) = self.top_p {
            top_p.to_bits().hash(state);
        }
        if let Some(freq) = self.frequency_penalty {
            freq.to_bits().hash(state);
        }
        if let Some(pres) = self.presence_penalty {
            pres.to_bits().hash(state);
        }
        self.stop_sequences.hash(state);
    }
}

impl Default for PromptParameters {
    fn default() -> Self {
        Self {
            max_tokens: Some(1000),
            temperature: Some(0.7),
            top_p: Some(1.0),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TextPrompt {
    pub content: String,
    pub role: PromptRole,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PromptRole {
    System,
    User,
    Assistant,
    Function,
}

// Additional prompt types...
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SystemPrompt {
    pub instructions: String,
    pub constraints: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserPrompt {
    pub query: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssistantPrompt {
    pub response: String,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionPrompt {
    pub function_name: String,
    pub arguments: BTreeMap<String, String>,
    pub description: Option<String>,
}

impl PromptItem {
    pub fn new(prompt_type: PromptType) -> Result<Self, PromptItemError> {
        let prompt_data = PromptData {
            prompt_type,
            content_attachments: Vec::new(),
            parameters: PromptParameters::default(),
            context: None,
            expected_output: None,
            tags: None,
            category: None,
            author: None,
            metadata: BTreeMap::new(),
        };

        let node = Node::new(prompt_data, None);
        Ok(PromptItem { node })
    }

    pub fn new_with_title(prompt_type: PromptType, title: String) -> Result<Self, PromptItemError> {
        let prompt_data = PromptData {
            prompt_type,
            content_attachments: Vec::new(),
            parameters: PromptParameters::default(),
            context: None,
            expected_output: None,
            tags: None,
            category: None,
            author: None,
            metadata: BTreeMap::new(),
        };

        let node = Node::new(prompt_data, Some(title));
        Ok(PromptItem { node })
    }

    // Getters and setters
    pub fn uuid(&self) -> Uuid { self.node.uuid }
    pub fn title(&self) -> Option<&String> { self.node.name.as_ref() }
    pub fn prompt_type(&self) -> &PromptType { &self.node.node.prompt_type }
    pub fn parameters(&self) -> &PromptParameters { &self.node.node.parameters }
    pub fn content_attachments(&self) -> &[Uuid] { &self.node.node.content_attachments }
    
    pub fn add_content_attachment(&mut self, content_id: Uuid) {
        self.node.node.content_attachments.push(content_id);
        self.node.modified = Utc::now();
    }

    pub fn set_parameters(&mut self, parameters: PromptParameters) {
        self.node.node.parameters = parameters;
        self.node.modified = Utc::now();
    }

    pub fn set_context(&mut self, context: Option<String>) {
        self.node.node.context = context;
        self.node.modified = Utc::now();
    }
}

#[derive(Debug, Clone, Error)]
pub enum PromptItemError {
    #[error("Invalid prompt configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_parameters_equality() {
        let params1 = PromptParameters {
            max_tokens: Some(100),
            temperature: Some(0.5),
            top_p: Some(0.9),
            frequency_penalty: Some(0.1),
            presence_penalty: Some(0.2),
            stop_sequences: Some(vec!["END".to_string()]),
        };

        let params2 = PromptParameters {
            max_tokens: Some(100),
            temperature: Some(0.5),
            top_p: Some(0.9),
            frequency_penalty: Some(0.1),
            presence_penalty: Some(0.2),
            stop_sequences: Some(vec!["END".to_string()]),
        };

        assert_eq!(params1, params2);
    }

    #[test]
    fn test_prompt_parameters_ordering() {
        let params1 = PromptParameters {
            max_tokens: Some(100),
            ..Default::default()
        };

        let params2 = PromptParameters {
            max_tokens: Some(200),
            ..Default::default()
        };

        assert!(params1 < params2);
    }

    #[test]
    fn test_text_prompt_creation() {
        let text_prompt = TextPrompt {
            content: "Hello, world!".to_string(),
            role: PromptRole::User,
        };

        let prompt_item = PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Test Prompt".to_string(),
        );

        assert!(prompt_item.is_ok());
        let item = prompt_item.unwrap();
        assert_eq!(item.title(), Some(&"Test Prompt".to_string()));
        
        match item.prompt_type() {
            PromptType::Text(text) => {
                assert_eq!(text.content, "Hello, world!");
                assert_eq!(text.role, PromptRole::User);
            },
            _ => panic!("Expected text prompt"),
        }
    }

    #[test]
    fn test_prompt_item_modifications() {
        let text_prompt = TextPrompt {
            content: "Test content".to_string(),
            role: PromptRole::System,
        };

        let mut prompt_item = PromptItem::new_with_title(
            PromptType::Text(text_prompt),
            "Modifiable Prompt".to_string(),
        ).unwrap();

        // Test adding content attachment
        let content_id = Uuid::new_v4();
        prompt_item.add_content_attachment(content_id);
        assert_eq!(prompt_item.content_attachments().len(), 1);
        assert_eq!(prompt_item.content_attachments()[0], content_id);

        // Test setting parameters
        let new_params = PromptParameters {
            max_tokens: Some(500),
            temperature: Some(0.8),
            ..Default::default()
        };
        prompt_item.set_parameters(new_params.clone());
        assert_eq!(prompt_item.parameters(), &new_params);

        // Test setting context
        prompt_item.set_context(Some("Test context".to_string()));
        assert_eq!(prompt_item.node.node.context, Some("Test context".to_string()));
    }
}