//! Test helpers and utilities
//!
//! Common test infrastructure including mocks, fixtures, and helper functions.

pub mod e2e_fixtures;
pub mod mock_arsenal_adapter;
pub mod mock_llm_adapter;
pub mod mock_paladin_port;

pub use mock_arsenal_adapter::MockArsenalPort;

pub use mock_llm_adapter::{
    Invocation, MockLlmAdapter, MockResponse, create_mock_with_mixed_responses,
    create_mock_with_responses, create_mock_with_tool_calls, create_test_paladin_with_mock,
};

pub use mock_paladin_port::{FaultyPaladinPort, MockPaladinPort};
