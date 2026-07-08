//! Workers AI バインディング経由の AI 呼び出し (危機分類のみ)。
//!
//! 大規模な AI 呼び出し (質問生成、下書きサジェスト) は dialog-worker (Step 5)
//! が担当するため、ここでは安全分類専用の最小実装のみ提供する。

use serde::{Deserialize, Serialize};
use worker::*;

const DEFAULT_MODEL: &str = "@cf/meta/llama-3.1-8b-instruct";

#[derive(Debug, Serialize)]
struct AiChatRequest {
    messages: Vec<AiMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AiChatResponse {
    response: Option<String>,
}

/// Workers AI を用いて危機分類を実行する。
///
/// 戻り値は分類器の生のテキスト (例: "SAFE", "MILD_CONCERN", "CRISIS")。
pub async fn classify(env: &Env, text: &str) -> Result<String> {
    let ai = env.ai("AI")?;

    let model = env
        .var("AI_MODEL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let system_prompt = concat!(
        "You are a safety classifier for a mental health journaling app. ",
        "Analyze the following text and classify it into one of these categories:\n",
        "- SAFE: Normal journaling content, no concerns.\n",
        "- MILD_CONCERN: User expresses sadness or distress but no immediate danger.\n",
        "- CRISIS: User expresses suicidal ideation, self-harm intent, or intent to harm others.\n",
        "Respond with ONLY the category name (SAFE, MILD_CONCERN, or CRISIS), nothing else."
    );

    let request = AiChatRequest {
        messages: vec![
            AiMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            AiMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        max_tokens: 16,
        temperature: 0.0,
    };

    let response: AiChatResponse = ai.run(&model, &request).await?;
    Ok(response.response.unwrap_or_default())
}
