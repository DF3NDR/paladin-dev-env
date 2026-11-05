# Live API Tests - Issues Fixed

## Problems Identified

### 1. Tests Were Silently "Passing" When API Keys Missing
**Before:** Tests printed "SKIPPED" and returned early, counting as PASS
**After:** Tests panic with clear error messages when API keys are missing/empty

### 2. Empty API Keys Treated as "Present"
**Before:** `DEEPSEEK_API_KEY=` returned `Ok("")`, then adapter panicked
**After:** `require_api_key()` checks for empty strings and fails with helpful message

### 3. .env File Not Loaded During Tests
**Before:** Only `cargo run` loaded .env via main.rs
**After:** Tests have their own `init_test_env()` that calls `dotenv::dotenv()`

### 4. Anthropic Tests Had Prompt Bug
**Before:** Sent SystemPrompt, Anthropic requires user messages
**After:** Changed to UserPrompt with `query` field

## How to Run Tests Now

### Method 1: Using .env file (Recommended)

```bash
# 1. Fix your .env file (see below)
vim .env

# 2. Tests auto-load .env
cargo test --features live-api-tests -- --ignored
```

### Method 2: Export variables

```bash
export OPENAI_API_KEY="sk-..."
export DEEPSEEK_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-..."

cargo test --features live-api-tests -- --ignored
```

## CRITICAL: Fix Your .env File

Your current .env has a **broken OPENAI_API_KEY** with a space:

```bash
# Current (BROKEN):
OPENAI_API_KEY=sk-proj-REDACTED REDACTED
#                               ↑ Invalid space breaks the key!

# Should be (FIXED - no spaces):
OPENAI_API_KEY=sk-proj-REDACTED
```

To fix:

1. Open .env in an editor
2. Find the OPENAI_API_KEY line
3. Remove the space (join the two parts)
4. Save the file

You can verify it's fixed:

```bash
grep "^OPENAI_API_KEY" .env | cat -A
# Should see ONE line ending with $, no spaces
```

## Test Behavior Now

### With Valid API Key
```bash
$ cargo test --features live-api-tests test_openai_basic_completion -- --ignored

running 1 test
✓ OpenAI basic completion: Hello from OpenAI
test integration::llm_live_api_tests::test_openai_basic_completion ... ok
```

### With Missing API Key
```bash
$ cargo test --features live-api-tests test_deepseek_basic_completion -- --ignored

thread '...' panicked at tests/integration/llm_live_api_tests.rs:74:
❌ DeepSeek API key is empty. Set DEEPSEEK_API_KEY in .env file or environment.

To skip this test, don't run with --ignored flag.
To run with a valid key: export DEEPSEEK_API_KEY="your-key-here"

test integration::llm_live_api_tests::test_deepseek_basic_completion ... FAILED
```

### Without --ignored Flag
```bash
$ cargo test --features live-api-tests test_openai_basic_completion

# Test is ignored, won't run at all (0 tests executed)
```

## Running Specific Provider Tests

```bash
# OpenAI only (4 tests)
cargo test --features live-api-tests test_openai -- --ignored

# DeepSeek only (4 tests)
cargo test --features live-api-tests test_deepseek -- --ignored

# Anthropic only (4 tests)
cargo test --features live-api-tests test_anthropic -- --ignored
```

## Summary of Changes

**File: `tests/integration/llm_live_api_tests.rs`**

1. Added `init_test_env()` function with `Once` to load .env once per test run
2. Changed `require_api_key()` to panic with helpful messages instead of returning Result
3. Fixed `create_test_prompt()` to use UserPrompt (Anthropic compatibility)
4. Removed all `match require_api_key()` error handling (now panics directly)
5. Updated documentation to reflect new behavior

## Next Steps

1. **Fix your .env file** - Remove the space in OPENAI_API_KEY
2. **Add DeepSeek key** - If you want to test DeepSeek, add a real key
3. **Run tests** - `cargo test --features live-api-tests -- --ignored --nocapture`
4. **Expect failures** - Tests will now properly FAIL when keys are missing

## Why This Is Better

**Before:**
- ❌ Tests "passed" with missing keys (confusing!)
- ❌ No clear error messages
- ❌ Had to manually load .env

**Now:**
- ✅ Tests fail loudly with clear error messages
- ✅ .env auto-loaded in tests
- ✅ Easy to see which keys are missing/empty
- ✅ Anthropic tests work properly with UserPrompt
