// SkyBridge - Anthropic-to-Ollama API Translator
// Translates between Anthropic Messages API format and Ollama API format
// for offline Sky-Code operation.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::post,
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, convert::Infallible, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

// ===========================
// Anthropic API Types (Input)
// ===========================

#[derive(Debug, Deserialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    tools: Vec<Tool>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: serde_json::Value,  // Can be string or array of content blocks
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Tool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: serde_json::Value,
}

// ===========================
// Ollama API Types (Output)
// ===========================

#[derive(Debug, Serialize, Clone)]
struct OllamaRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    messages: Option<Vec<OllamaMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Debug, Serialize, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,  // "function"
    function: OllamaFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,  // "function"
    function: OllamaFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize, Clone)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
   #[serde(default)]
    message: Option<OllamaResponseMessage>,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

// ===========================
// Ollama /api/generate API Types (Legacy - for Ollama 0.20.x)
// ===========================

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    done: bool,
}

// ===========================
// Anthropic SSE Event Types
// ===========================

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        message: MessageStartPayload,
    },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: ContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        index: u32,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaPayload,
        usage: Usage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
}

#[derive(Debug, Serialize)]
struct MessageStartPayload {
    id: String,
    #[serde(rename = "type")]
    message_type: String,
    role: String,
    content: Vec<serde_json::Value>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Usage,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[allow(dead_code)]
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
}

#[derive(Debug, Serialize)]
struct MessageDeltaPayload {
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

// ===========================
// Application State
// ===========================

struct AppState {
    ollama_base_url: String,
    http_client: reqwest::Client,
}

// ===========================
// Main Server
// ===========================

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("skybridge=info,tower_http=debug")
        .init();

    let ollama_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    info!("🌉 SkyBridge starting...");
    info!("   Anthropic-compatible API: http://0.0.0.0:4000");
    info!("   Ollama Backend: {}", ollama_url);

    let state = Arc::new(AppState {
        ollama_base_url: ollama_url,
        http_client: reqwest::Client::new(),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/v1/messages", post(messages_handler))
        .route("/health", post(health_handler))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000")
        .await
        .expect("Failed to bind to port 4000");

    info!("✅ SkyBridge ready on port 4000");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

// ===========================
// Health Check Handler
// ===========================

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

// ===========================
// Messages Handler
// ===========================

async fn messages_handler(
    State(state): State<Arc<AppState>>,
    _headers: HeaderMap,
    Json(request): Json<AnthropicRequest>,
) -> Response {
    info!(
        "📨 Received request: model={}, stream={}, max_tokens={:?}",
        request.model,
        request.stream,
        request.max_tokens
    );

    // Translate Anthropic → Ollama
    let ollama_req = translate_request(&request);
    let system = request.system.clone();

    if request.stream {
        handle_streaming(state, ollama_req, request.model, system).await
    } else {
        handle_non_streaming(state, ollama_req, system).await
    }
}

// ===========================
// Request Translation
// ===========================

fn translate_request(req: &AnthropicRequest) -> OllamaRequest {
    let mut messages: Vec<OllamaMessage> = req
        .messages
        .iter()
        .map(|m| {
            // Extract text and tool_calls from content
            let (content_text, tool_calls) = match &m.content {
                serde_json::Value::String(s) => (s.clone(), None),
                serde_json::Value::Array(blocks) => {
                    let mut texts = Vec::new();
                    let mut calls = Vec::new();

                    for block in blocks {
                        if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                            match block_type {
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        texts.push(text.to_string());
                                    }
                                }
                                "tool_use" => {
                                    // Extract tool_use block
                                    if let (Some(id), Some(name), Some(input)) = (
                                        block.get("id").and_then(|i| i.as_str()),
                                        block.get("name").and_then(|n| n.as_str()),
                                        block.get("input"),
                                    ) {
                                        calls.push(OllamaToolCall {
                                            id: id.to_string(),
                                            call_type: "function".to_string(),
                                            function: OllamaFunctionCall {
                                                name: name.to_string(),
                                                arguments: input.to_string(),
                                            },
                                        });
                                    }
                                }
                                "tool_result" => {
                                    // Tool results are in user messages in Anthropic format
                                    if let (Some(tool_use_id), Some(output)) = (
                                        block.get("tool_use_id").and_then(|i| i.as_str()),
                                        block.get("output").and_then(|o| o.as_str()),
                                    ) {
                                        texts.push(format!("[Tool Result for {}]: {}", tool_use_id, output));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    let text = texts.join("\n");
                    (text, if calls.is_empty() { None } else { Some(calls) })
                }
                _ => (String::new(), None),
            };
            
            OllamaMessage {
                role: m.role.clone(),
                content: content_text,
                tool_calls,
            }
        })
        .collect();

    // Add system message if present
    if let Some(ref system) = req.system {
        messages.insert(
            0,
            OllamaMessage {
                role: "system".to_string(),
                content: system.clone(),
                tool_calls: None,
            },
        );
    }

    // Translate tools
    let ollama_tools = if req.tools.is_empty() {
        None
    } else {
        Some(
            req.tools
                .iter()
                .map(|t| OllamaTool {
                    tool_type: "function".to_string(),
                    function: OllamaFunction {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.input_schema.clone(),
                    },
                })
                .collect(),
        )
    };

    let mapped_model = map_model_name(&req.model);
    // Strip tools for models that don't support native tool calling
    // to prevent malformed/empty responses
    let tools = if model_supports_tools(&mapped_model) {
        ollama_tools
    } else {
        None
    };

    OllamaRequest {
        model: mapped_model,
        messages: Some(messages),
        options: req.max_tokens.map(|mt| OllamaOptions {
            num_predict: Some(mt),
        }),
        stream: req.stream,
        tools,
    }
}

fn map_model_name(anthropic_model: &str) -> String {
    // Allow override via OLLAMA_MODEL env var
    if let Ok(model) = std::env::var("OLLAMA_MODEL") {
        if !model.is_empty() {
            return model;
        }
    }
    // Map Anthropic-compatible model names to Ollama models
    match anthropic_model {
        "cloud-apus-4-6" | "cloud-sannet-4-6" | "cloud-haiku-4" => "llama3.2:1b".to_string(),
        // Backward compat: old sky-* aliases still work
        "sky-apus-4-6" | "sky-sannet-4-6" | "sky-haiku-4" => "llama3.2:1b".to_string(),
        // Pass through any real Ollama model names directly (sky-uncensored, llama3.1:8b, etc.)
        name if !name.starts_with("cloud-") && !name.starts_with("sky-") => name.to_string(),
        name if name == "sky-uncensored" => name.to_string(),
        _ => "llama3.2:1b".to_string(),
    }
}

/// Returns true if the model supports native tool calling via Ollama
fn model_supports_tools(model: &str) -> bool {
    // Only models specifically known to support tool calling reliably
    // llama3.2:1b does NOT support tools - it generates malformed responses
    let tool_capable = ["qwen2.5", "mistral", "llama3.1", "llama3.3", "llama3.2:3b", "llama3.2:8b"];
    // sky-uncensored is based on llama3.2:1b - no tool support
    if model == "sky-uncensored" { return false; }
    tool_capable.iter().any(|prefix| model.starts_with(prefix))
}

// Convert messages array to a single prompt string for /api/generate
fn messages_to_prompt(messages: &Option<Vec<OllamaMessage>>, system: &Option<String>) -> String {
    let mut prompt = String::new();
    
    // Add system message if present
    if let Some(sys) = system {
        prompt.push_str("System: ");
        prompt.push_str(sys);
        prompt.push_str("\n\n");
    }
    
    // Convert message history to prompt
    for msg in messages.as_deref().unwrap_or(&[]) {
        match msg.role.as_str() {
            "user" => {
                prompt.push_str("User: ");
                prompt.push_str(&msg.content);
                prompt.push_str("\n\n");
            }
            "assistant" => {
                prompt.push_str("Assistant: ");
                prompt.push_str(&msg.content);
                prompt.push_str("\n\n");
            }
            _ => {}
        }
    }
    
    // Add final assistant prefix to elicit response
    prompt.push_str("Assistant:");
    
    prompt
}

// ===========================
// Non-Streaming Handler
// ===========================

async fn handle_non_streaming(
    state: Arc<AppState>,
    ollama_req: OllamaRequest,
    system: Option<String>,
) -> Response {
    // Try modern /api/chat first (Ollama 0.5.x+)
    let chat_url = format!("{}/api/chat", state.ollama_base_url);
    
    match state.http_client.post(&chat_url).json(&ollama_req).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<OllamaResponse>().await {
                Ok(ollama_resp) => {
                    info!("Using /api/chat endpoint (modern Ollama)");
                    let anthropic_resp = translate_non_streaming_response(ollama_resp);
                    return Json(anthropic_resp).into_response();
                }
                Err(e) => {
                    error!("Failed to parse /api/chat response: {}", e);
                }
            }
        }
        Ok(resp) => {
            info!("/api/chat returned {}, trying /api/generate fallback", resp.status());
        }
        Err(e) => {
            info!("/api/chat failed ({}), trying /api/generate fallback", e);
        }
    }
    
    // Fallback to legacy /api/generate (Ollama 0.20.x)
    info!("Falling back to /api/generate endpoint (legacy Ollama)");
    let generate_url = format!("{}/api/generate", state.ollama_base_url);
    
    let prompt = messages_to_prompt(&ollama_req.messages, &system);
    let generate_req = OllamaGenerateRequest {
        model: ollama_req.model,
        prompt,
        stream: false,
        options: ollama_req.options,
    };
    
    match state.http_client.post(&generate_url).json(&generate_req).send().await {
        Ok(resp) => match resp.json::<OllamaGenerateResponse>().await {
            Ok(ollama_resp) => {
                let anthropic_resp = translate_generate_response(ollama_resp);
                Json(anthropic_resp).into_response()
            }
            Err(e) => {
                error!("Failed to parse /api/generate response: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Parse error").into_response()
            }
        },
        Err(e) => {
            error!("Failed to call /api/generate: {}", e);
            (StatusCode::BAD_GATEWAY, "Ollama unavailable").into_response()
        }
    }
}

fn translate_non_streaming_response(ollama: OllamaResponse) -> serde_json::Value {
    let message = ollama.message.unwrap_or(OllamaResponseMessage {
        role: "assistant".to_string(),
        content: String::new(),
        tool_calls: None,
    });

    let mut content_blocks: Vec<serde_json::Value> = Vec::new();

    // Add text content if present
    if !message.content.is_empty() {
        content_blocks.push(serde_json::json!({
            "type": "text",
            "text": message.content
        }));
    }

    // Add tool_use blocks if present
    if let Some(tool_calls) = message.tool_calls {
        for call in tool_calls {
            // Parse arguments back to JSON
            let input: serde_json::Value = serde_json::from_str(&call.function.arguments)
                .unwrap_or(serde_json::json!({}));

            content_blocks.push(serde_json::json!({
                "type": "tool_use",
                "id": call.id,
                "name": call.function.name,
                "input": input
            }));
        }
    }

    serde_json::json!({
        "id": "msg_skybridge",
        "type": "message",
        "role": "assistant",
        "content": content_blocks,
        "model": std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
        "stop_reason": if content_blocks.iter().any(|b| b["type"] == "tool_use") { "tool_use" } else { "end_turn" },
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0
        }
    })
}

// Translate /api/generate response to Anthropic format
fn translate_generate_response(ollama: OllamaGenerateResponse) -> serde_json::Value {
    let content_blocks = vec![serde_json::json!({
        "type": "text",
        "text": ollama.response
    })];

    serde_json::json!({
        "id": "msg_skybridge_generate",
        "type": "message",
        "role": "assistant",
        "content": content_blocks,
        "model": std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0
        }
    })
}

// ===========================
// Streaming Handler
// ===========================

async fn handle_streaming(
    state: Arc<AppState>,
    ollama_req: OllamaRequest,
    model: String,
    system: Option<String>,
) -> Response {
    let chat_url = format!("{}/api/chat", state.ollama_base_url);

    // Try /api/chat first (Ollama 0.5+)
    // Send a probe non-streaming request to check if /api/chat works
    let probe_req = OllamaRequest {
        model: ollama_req.model.clone(),
        messages: ollama_req.messages.clone(),
        options: ollama_req.options.clone(),
        stream: false,
        tools: None,
    };

    match state.http_client.post(&chat_url).json(&probe_req).send().await {
        Ok(resp) if resp.status().is_success() => {
            // /api/chat responded — use it for streaming too
            let stream_req = OllamaRequest { stream: true, ..ollama_req.clone() };
            match state.http_client.post(&chat_url).json(&stream_req).send().await {
                Ok(stream_resp) => {
                    let stream = translate_streaming_response(stream_resp, model);
                    // NO keepalive — it causes "error decoding response body" in reqwest
                    Sse::new(stream).into_response()
                }
                Err(e) => {
                    error!("Streaming /api/chat failed: {}", e);
                    stream_via_generate(state, ollama_req, model, system).await
                }
            }
        }
        _ => {
            // Fallback: use /api/generate for streaming (Ollama 0.20.x)
            info!("Falling back to /api/generate for streaming (legacy Ollama)");
            stream_via_generate(state, ollama_req, model, system).await
        }
    }
}

/// Stream using legacy /api/generate endpoint (works with Ollama 0.20.x)
async fn stream_via_generate(
    state: Arc<AppState>,
    ollama_req: OllamaRequest,
    model: String,
    system: Option<String>,
) -> Response {
    let generate_url = format!("{}/api/generate", state.ollama_base_url);
    let prompt = messages_to_prompt(&ollama_req.messages, &system);
    let generate_req = OllamaGenerateRequest {
        model: ollama_req.model.clone(),
        prompt,
        stream: true,
        options: ollama_req.options,
    };

    match state.http_client.post(&generate_url).json(&generate_req).send().await {
        Ok(resp) => {
            let stream = translate_generate_streaming_response(resp, model);
            Sse::new(stream).into_response()
        }
        Err(e) => {
            error!("Failed to call Ollama /api/generate streaming: {}", e);
            (StatusCode::BAD_GATEWAY, "Ollama unavailable").into_response()
        }
    }
}

fn translate_streaming_response(
    response: reqwest::Response,
    model: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let byte_stream = response.bytes_stream();

    stream::unfold(
        (
            byte_stream,
            false,
            false,
            0u32,
            model,
            Vec::<u8>::new(),
            VecDeque::<(String, String)>::new(),
            false,
        ),
        |(
            mut stream,
            mut start_sent,
            mut block_sent,
            mut tokens,
            model,
            mut line_buffer,
            mut pending,
            mut finalized,
        )| async move {
            use futures::StreamExt;

            // Emit queued events first.
            if let Some((event_name, data)) = pending.pop_front() {
                return Some((
                    Ok(Event::default().event(event_name).data(data)),
                    (
                        stream,
                        start_sent,
                        block_sent,
                        tokens,
                        model,
                        line_buffer,
                        pending,
                        finalized,
                    ),
                ));
            }

            if finalized {
                return None;
            }

            // Process incoming chunks and queue all generated events.
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        line_buffer.extend_from_slice(&chunk);

                        while let Some(newline_idx) = line_buffer.iter().position(|b| *b == b'\n') {
                            let mut line_bytes: Vec<u8> = line_buffer.drain(..=newline_idx).collect();
                            if line_bytes.last() == Some(&b'\n') {
                                line_bytes.pop();
                            }
                            if line_bytes.last() == Some(&b'\r') {
                                line_bytes.pop();
                            }

                            if line_bytes.is_empty() {
                                continue;
                            }

                            // Parse one NDJSON frame from Ollama.
                            if let Ok(ollama_resp) = serde_json::from_slice::<OllamaResponse>(&line_bytes) {
                                if !start_sent {
                                    let event = AnthropicStreamEvent::MessageStart {
                                        message: MessageStartPayload {
                                            id: "msg_skybridge".to_string(),
                                            message_type: "message".to_string(),
                                            role: "assistant".to_string(),
                                            content: vec![],
                                            model: model.clone(),
                                            stop_reason: None,
                                            stop_sequence: None,
                                            usage: Usage {
                                                input_tokens: 0,
                                                output_tokens: 0,
                                            },
                                        },
                                    };
                                    start_sent = true;
                                    pending.push_back((
                                        "message_start".to_string(),
                                        serde_json::to_string(&event).unwrap(),
                                    ));
                                }

                                if !block_sent {
                                    let event = AnthropicStreamEvent::ContentBlockStart {
                                        index: 0,
                                        content_block: ContentBlock::Text {
                                            text: String::new(),
                                        },
                                    };
                                    block_sent = true;
                                    pending.push_back((
                                        "content_block_start".to_string(),
                                        serde_json::to_string(&event).unwrap(),
                                    ));
                                }

                                if let Some(msg) = ollama_resp.message {
                                    if !msg.content.is_empty() {
                                        tokens += 1;

                                        let event = AnthropicStreamEvent::ContentBlockDelta {
                                            index: 0,
                                            delta: ContentDelta::TextDelta {
                                                text: msg.content,
                                            },
                                        };
                                        pending.push_back((
                                            "content_block_delta".to_string(),
                                            serde_json::to_string(&event).unwrap(),
                                        ));
                                    }
                                }

                                if ollama_resp.done {
                                    pending.push_back((
                                        "content_block_stop".to_string(),
                                        serde_json::to_string(&AnthropicStreamEvent::ContentBlockStop { index: 0 }).unwrap(),
                                    ));

                                    let delta_event = AnthropicStreamEvent::MessageDelta {
                                        delta: MessageDeltaPayload {
                                            stop_reason: Some("end_turn".to_string()),
                                            stop_sequence: None,
                                        },
                                        usage: Usage {
                                            input_tokens: 0,
                                            output_tokens: tokens,
                                        },
                                    };
                                    pending.push_back((
                                        "message_delta".to_string(),
                                        serde_json::to_string(&delta_event).unwrap(),
                                    ));

                                    pending.push_back((
                                        "message_stop".to_string(),
                                        serde_json::to_string(&AnthropicStreamEvent::MessageStop)
                                            .unwrap(),
                                    ));

                                    finalized = true;
                                }
                            }
                        }

                        if let Some((event_name, data)) = pending.pop_front() {
                            return Some((
                                Ok(Event::default().event(event_name).data(data)),
                                (
                                    stream,
                                    start_sent,
                                    block_sent,
                                    tokens,
                                    model,
                                    line_buffer,
                                    pending,
                                    finalized,
                                ),
                            ));
                        }
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        break;
                    }
                }
            }

            // If the stream ended without a trailing newline, process the final frame.
            if !line_buffer.is_empty() && !line_buffer.iter().all(|b| b.is_ascii_whitespace()) {
                if let Ok(ollama_resp) = serde_json::from_slice::<OllamaResponse>(&line_buffer) {
                    if !start_sent {
                        let event = AnthropicStreamEvent::MessageStart {
                            message: MessageStartPayload {
                                id: "msg_skybridge".to_string(),
                                message_type: "message".to_string(),
                                role: "assistant".to_string(),
                                content: vec![],
                                model: model.clone(),
                                stop_reason: None,
                                stop_sequence: None,
                                usage: Usage {
                                    input_tokens: 0,
                                    output_tokens: 0,
                                },
                            },
                        };
                        start_sent = true;
                        pending.push_back((
                            "message_start".to_string(),
                            serde_json::to_string(&event).unwrap(),
                        ));
                    }

                    if !block_sent {
                        let event = AnthropicStreamEvent::ContentBlockStart {
                            index: 0,
                            content_block: ContentBlock::Text {
                                text: String::new(),
                            },
                        };
                        block_sent = true;
                        pending.push_back((
                            "content_block_start".to_string(),
                            serde_json::to_string(&event).unwrap(),
                        ));
                    }

                    if let Some(msg) = ollama_resp.message {
                        if !msg.content.is_empty() {
                            tokens += 1;
                            let event = AnthropicStreamEvent::ContentBlockDelta {
                                index: 0,
                                delta: ContentDelta::TextDelta { text: msg.content },
                            };
                            pending.push_back((
                                "content_block_delta".to_string(),
                                serde_json::to_string(&event).unwrap(),
                            ));
                        }
                    }

                    if ollama_resp.done && !finalized {
                        pending.push_back((
                            "content_block_stop".to_string(),
                            serde_json::to_string(&AnthropicStreamEvent::ContentBlockStop { index: 0 })
                                .unwrap(),
                        ));

                        let delta_event = AnthropicStreamEvent::MessageDelta {
                            delta: MessageDeltaPayload {
                                stop_reason: Some("end_turn".to_string()),
                                stop_sequence: None,
                            },
                            usage: Usage {
                                input_tokens: 0,
                                output_tokens: tokens,
                            },
                        };
                        pending.push_back((
                            "message_delta".to_string(),
                            serde_json::to_string(&delta_event).unwrap(),
                        ));

                        pending.push_back((
                            "message_stop".to_string(),
                            serde_json::to_string(&AnthropicStreamEvent::MessageStop).unwrap(),
                        ));

                        finalized = true;
                    }
                }
            }

            // Stream ended unexpectedly: close open Anthropic stream cleanly.
            if !finalized && start_sent && block_sent {
                pending.push_back((
                    "content_block_stop".to_string(),
                    serde_json::to_string(&AnthropicStreamEvent::ContentBlockStop { index: 0 })
                        .unwrap(),
                ));

                let delta_event = AnthropicStreamEvent::MessageDelta {
                    delta: MessageDeltaPayload {
                        stop_reason: Some("end_turn".to_string()),
                        stop_sequence: None,
                    },
                    usage: Usage {
                        input_tokens: 0,
                        output_tokens: tokens,
                    },
                };
                pending.push_back((
                    "message_delta".to_string(),
                    serde_json::to_string(&delta_event).unwrap(),
                ));

                pending.push_back((
                    "message_stop".to_string(),
                    serde_json::to_string(&AnthropicStreamEvent::MessageStop).unwrap(),
                ));

                finalized = true;

                if let Some((event_name, data)) = pending.pop_front() {
                    return Some((
                        Ok(Event::default().event(event_name).data(data)),
                        (
                            stream,
                            start_sent,
                            block_sent,
                            tokens,
                            model,
                            line_buffer,
                            pending,
                            finalized,
                        ),
                    ));
                }
            }

            None
        },
    )
}

/// Translate /api/generate NDJSON streaming into Anthropic SSE format
fn translate_generate_streaming_response(
    response: reqwest::Response,
    model: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let byte_stream = response.bytes_stream();

    stream::unfold(
        (byte_stream, false, false, 0u32, model, Vec::<u8>::new(), VecDeque::<(String, String)>::new(), false),
        |(mut stream, mut start_sent, mut block_sent, mut tokens, model, mut line_buffer, mut pending, mut finalized)| async move {
            use futures::StreamExt;

            if let Some((event_name, data)) = pending.pop_front() {
                return Some((
                    Ok(Event::default().event(event_name).data(data)),
                    (stream, start_sent, block_sent, tokens, model, line_buffer, pending, finalized),
                ));
            }

            if finalized { return None; }

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        line_buffer.extend_from_slice(&chunk);

                        while let Some(nl) = line_buffer.iter().position(|b| *b == b'\n') {
                            let mut line_bytes: Vec<u8> = line_buffer.drain(..=nl).collect();
                            if line_bytes.last() == Some(&b'\n') { line_bytes.pop(); }
                            if line_bytes.last() == Some(&b'\r') { line_bytes.pop(); }
                            if line_bytes.is_empty() { continue; }

                            if let Ok(gen_resp) = serde_json::from_slice::<OllamaGenerateResponse>(&line_bytes) {
                                if !start_sent {
                                    let ev = AnthropicStreamEvent::MessageStart {
                                        message: MessageStartPayload {
                                            id: "msg_skybridge_generate".to_string(),
                                            message_type: "message".to_string(),
                                            role: "assistant".to_string(),
                                            content: vec![],
                                            model: model.clone(),
                                            stop_reason: None,
                                            stop_sequence: None,
                                            usage: Usage { input_tokens: 0, output_tokens: 0 },
                                        },
                                    };
                                    start_sent = true;
                                    pending.push_back(("message_start".to_string(), serde_json::to_string(&ev).unwrap()));
                                }

                                if !block_sent {
                                    let ev = AnthropicStreamEvent::ContentBlockStart {
                                        index: 0,
                                        content_block: ContentBlock::Text { text: String::new() },
                                    };
                                    block_sent = true;
                                    pending.push_back(("content_block_start".to_string(), serde_json::to_string(&ev).unwrap()));
                                }

                                if !gen_resp.response.is_empty() {
                                    tokens += 1;
                                    let ev = AnthropicStreamEvent::ContentBlockDelta {
                                        index: 0,
                                        delta: ContentDelta::TextDelta { text: gen_resp.response },
                                    };
                                    pending.push_back(("content_block_delta".to_string(), serde_json::to_string(&ev).unwrap()));
                                }

                                if gen_resp.done {
                                    pending.push_back(("content_block_stop".to_string(),
                                        serde_json::to_string(&AnthropicStreamEvent::ContentBlockStop { index: 0 }).unwrap()));
                                    let delta_ev = AnthropicStreamEvent::MessageDelta {
                                        delta: MessageDeltaPayload { stop_reason: Some("end_turn".to_string()), stop_sequence: None },
                                        usage: Usage { input_tokens: 0, output_tokens: tokens },
                                    };
                                    pending.push_back(("message_delta".to_string(), serde_json::to_string(&delta_ev).unwrap()));
                                    pending.push_back(("message_stop".to_string(),
                                        serde_json::to_string(&AnthropicStreamEvent::MessageStop).unwrap()));
                                    finalized = true;
                                }
                            }
                        }

                        if let Some((event_name, data)) = pending.pop_front() {
                            return Some((
                                Ok(Event::default().event(event_name).data(data)),
                                (stream, start_sent, block_sent, tokens, model, line_buffer, pending, finalized),
                            ));
                        }
                    }
                    Err(e) => {
                        error!("Generate stream error: {}", e);
                        break;
                    }
                }
            }

            None
        },
    )
}
