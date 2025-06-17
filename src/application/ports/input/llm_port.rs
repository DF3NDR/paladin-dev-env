/*

LLM Port

A port that defines how the application interacts with the LLM (Low Level Model).

This port is responsible for translating high-level application use cases into interactions
with the LLM. It provides an abstraction layer that allows the application to interact with
the LLM without being tightly coupled to its implementation details.

Typical implementations of this port would be for the adapter to translate the requirements
of the high-level use cases into calls to the LLM, and to translate the results of those calls
back into a format that the application can use.

An LLM Api usually requires a few standard fields to be present in the request and response
like the prompt, max_tokens, different weights for "temperature". The LLM Port handles 
these fields and provide a clean interface for the application to interact with the LLM and
for the adapter to translate the application's requirements into calls to the LLM.

*/
use reqwest::Error as ReqwestError;
use crate::core::platform::container::content::ContentItem;
use crate::core::platform::container::prompt::PromptItem;

/// This is a very simple example of an LLM Port that generates text based on a prompt.
pub trait LlmPort {
    fn prompt_llm(&self, messages: LlmMessage) -> Result<LlmThreadResponse, LlmError>;
    
    fn create_thread(&self, messages: LlmMessage) -> Result<LlmThreadResponse, LlmError>;
}

pub enum LlmError {
    ReqwestError(ReqwestError),
    InvalidPrompt,
    InvalidMaxTokens,
    InvalidCreativity,
}

impl From<ReqwestError> for LlmError {
    fn from(error: ReqwestError) -> Self {
        LlmError::ReqwestError(error)
    }
}

pub struct LlmMessage {
    pub role: LlmRole,
    pub prompt: PromptItem,
    pub max_tokens: u32,
    pub temperature: f32,
    pub attchments: Vec<ContentItem>,
}

pub enum LlmRole {
    User,
}

pub struct LlmThreadResponse {
    pub id: String,
    pub object: String,
    pub created_at: String,
    // A key value pair, Keys are max 64 characters, values are max 512 characters
    pub metadata: Vec<(String, String)>,
    // https://platform.openai.com/docs/api-reference/threads/createThread#threads-createthread-tool_resources
    pub tool_resources: Vec<String>,
}
