//! Workers AI ネイティブバインディング経由の AI 呼び出し。
//! 質問生成と下書きサジェスト。

use contracts::dialog::{AnswerType, DraftOption, QuestionPayload};
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

/// AI 呼び出しの低レベルラッパ
async fn call_ai(env: &Env, system_prompt: &str, user_prompt: &str) -> Result<String> {
    let ai = env.ai("AI")?;
    let model = env
        .var("AI_MODEL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let request = AiChatRequest {
        messages: vec![
            AiMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            AiMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ],
        max_tokens: 512,
        temperature: 0.7,
    };

    let response: AiChatResponse = ai.run(&model, &request).await?;
    Ok(response.response.unwrap_or_default())
}

// ────────────────────────────────────────────
// 質問生成
// ────────────────────────────────────────────

pub async fn generate_first_question(env: &Env, language: &str) -> Result<QuestionPayload> {
    let system = super::prompts::interview_system(language);
    let user_prompt = super::prompts::first_question_user_prompt(language);
    let response = call_ai(env, &system, user_prompt).await?;
    Ok(QuestionPayload {
        turn_id: uuid::Uuid::new_v4().to_string(),
        turn_order: 1,
        question: response.trim().to_string(),
        answer_type: AnswerType::Free,
        choices: None,
    })
}

pub async fn generate_next_question(
    env: &Env,
    language: &str,
    history: &[(String, String)],
    turn_order: i32,
) -> Result<QuestionPayload> {
    let system = super::prompts::interview_system(language);
    let history_text = history
        .iter()
        .map(|(q, a)| format!("Q: {}\nA: {}", q, a))
        .collect::<Vec<_>>()
        .join("\n---\n");
    let user_prompt = super::prompts::next_question_user_prompt(language, &history_text);
    let response = call_ai(env, &system, &user_prompt).await?;

    // 5 ターンごとに選択肢形式
    let (answer_type, choices) = if turn_order % 5 == 0 {
        (
            AnswerType::Scale,
            Some(vec![
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
            ]),
        )
    } else {
        (AnswerType::Free, None)
    };

    Ok(QuestionPayload {
        turn_id: uuid::Uuid::new_v4().to_string(),
        turn_order,
        question: response.trim().to_string(),
        answer_type,
        choices,
    })
}

// ────────────────────────────────────────────
// 下書きサジェスト
// ────────────────────────────────────────────

pub async fn generate_drafts(
    env: &Env,
    language: &str,
    user_input: &str,
    max_chars: usize,
) -> Result<Vec<DraftOption>> {
    let system = super::prompts::drafts_system(language, max_chars);
    let response = call_ai(env, &system, user_input).await?;

    let styles = ["reflective", "gratitude", "factual"];
    let drafts: Vec<DraftOption> = response
        .split("---")
        .filter(|s| !s.trim().is_empty())
        .enumerate()
        .map(|(i, text)| {
            let mut t = text.trim().to_string();
            if t.len() > max_chars {
                t.truncate(max_chars);
            }
            DraftOption {
                id: uuid::Uuid::new_v4().to_string(),
                text: t,
                style: styles.get(i).unwrap_or(&"other").to_string(),
            }
        })
        .take(3)
        .collect();

    Ok(drafts)
}
