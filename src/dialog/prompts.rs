//! AI システムプロンプトおよびユーザープロンプトの 5 言語テンプレート。
//!
//! ## レビュー状況 (2026-05 時点)
//!
//! | 言語 | システムプロンプト | first_question | next_question | drafts | 備考 |
//! |---|---|---|---|---|---|
//! | ja | reviewed | reviewed | reviewed | reviewed | プロジェクト発足時の主言語 |
//! | en | reviewed | reviewed | reviewed | reviewed | グローバル展開のベース |
//! | ar | not yet | not yet | not yet | not yet | 専門家監修待ち |
//! | uk | not yet | not yet | not yet | not yet | 専門家監修待ち |
//! | es | not yet | not yet | not yet | not yet | 専門家監修待ち |
//!
//! 暫定翻訳は **意味としては英語版と等価になるよう機械翻訳ベースで作成** されている。
//! 各言語のメンタルヘルス専門家による監修フローについては
//! `docs/i18n-review-flow.md` を参照。
//!
//! ## 設計
//!
//! プロンプト本体は `match language` で 5 分岐。`format!` で動的データ
//! (`history_text`、`max_chars`) を埋め込む。Project Fluent には移行しない:
//!
//! - システムプロンプトは 100〜500 文字の長尺テキストで Fluent の引数置換が
//!   ほぼ活きない (テキストブロック全体を分岐)
//! - 翻訳者にとって `.ftl` ファイルより Rust `match` 文の方が文脈が見やすい
//! - WASM サイズへの影響もごくわずか

/// `Accept-Language` 等から決まった言語コードに基づいて、
/// 翻訳済みの言語に正規化する。未対応言語は `"en"` にフォールバック。
fn normalize_lang(language: &str) -> &str {
    match language {
        "ja" | "en" | "ar" | "uk" | "es" => language,
        _ => "en",
    }
}

/// インタビュー (質問生成) 全体のシステムプロンプト。
pub fn interview_system(language: &str) -> String {
    match normalize_lang(language) {
        "ja" => "あなたはトラウマケアに配慮したジャーナリング支援アシスタントです。\n\
                 ユーザーの思考の整理を穏やかに手助けする質問を生成します。\n\
                 以下のルールを厳守してください：\n\
                 1. 医療的アドバイスは行わない\n\
                 2. ユーザーを批判・判断しない\n\
                 3. 穏やかで具体的な質問をする\n\
                 4. 質問は1つだけ、短く簡潔に".to_string(),
        "en" => "You are a trauma-aware journaling assistant.\n\
                  Generate gentle questions to help users organize their thoughts.\n\
                  Rules:\n\
                  1. Never give medical advice\n\
                  2. Never judge or criticize the user\n\
                  3. Ask gentle, specific questions\n\
                  4. One question only, keep it short".to_string(),
        // ──────────────────────────────────────────────────────────────
        // 以下、暫定翻訳 (専門家監修前)
        // ──────────────────────────────────────────────────────────────
        "ar" => "أنت مساعد كتابة يوميات يراعي الصدمات النفسية.\n\
                  قم بإنشاء أسئلة لطيفة لمساعدة المستخدمين على تنظيم أفكارهم.\n\
                  القواعد:\n\
                  1. لا تقدم نصيحة طبية أبداً\n\
                  2. لا تحكم أو تنتقد المستخدم\n\
                  3. اطرح أسئلة لطيفة ومحددة\n\
                  4. سؤال واحد فقط، اجعله قصيراً".to_string(),
        "uk" => "Ви — асистент для ведення щоденника, який враховує травматичний досвід.\n\
                  Генеруйте делікатні запитання, щоб допомогти користувачам впорядкувати свої думки.\n\
                  Правила:\n\
                  1. Ніколи не давайте медичних порад\n\
                  2. Ніколи не засуджуйте та не критикуйте користувача\n\
                  3. Ставте делікатні, конкретні запитання\n\
                  4. Тільки одне запитання, коротко".to_string(),
        "es" => "Eres un asistente de diario sensible al trauma.\n\
                  Genera preguntas amables para ayudar a las personas a organizar sus pensamientos.\n\
                  Reglas:\n\
                  1. Nunca des consejos médicos\n\
                  2. Nunca juzgues ni critiques a la persona\n\
                  3. Haz preguntas amables y específicas\n\
                  4. Solo una pregunta, breve".to_string(),
        _ => unreachable!("normalize_lang ensures a known code"),
    }
}

/// 最初の質問を生成するためのユーザー側プロンプト。
pub fn first_question_user_prompt(language: &str) -> &'static str {
    match normalize_lang(language) {
        "ja" => {
            "新しいジャーナリングセッションを始めます。ユーザーが安心して書き始められるような、\
                 穏やかで具体的な最初の質問を1つ生成してください。質問のみを出力してください。"
        }
        "en" => {
            "Start a new journaling session. Generate one gentle, specific first question \
                  that helps the user feel safe to begin writing. Output only the question."
        }
        "ar" => {
            "ابدأ جلسة كتابة يوميات جديدة. اكتب سؤالاً واحداً لطيفاً ومحدداً يساعد المستخدم \
                  على الشعور بالأمان لبدء الكتابة. أخرج السؤال فقط."
        }
        "uk" => {
            "Розпочніть новий сеанс ведення щоденника. Згенеруйте одне делікатне, конкретне перше запитання, \
                  яке допоможе користувачу відчути себе в безпеці, щоб почати писати. Виведіть лише запитання."
        }
        "es" => {
            "Inicia una nueva sesión de diario. Genera una primera pregunta amable y específica \
                  que ayude a la persona a sentirse segura para empezar a escribir. Devuelve solo la pregunta."
        }
        _ => unreachable!(),
    }
}

/// 続きの質問を生成するためのユーザー側プロンプトテンプレート。
/// `{history}` プレースホルダに対話履歴を `format!` で埋め込む。
pub fn next_question_user_prompt(language: &str, history_text: &str) -> String {
    match normalize_lang(language) {
        "ja" => format!(
            "以下はこれまでの対話履歴です：\n{}\n\n\
             上記を踏まえ、ユーザーの思考をさらに深掘りする次の質問を1つ生成してください。\
             質問のみを出力してください。",
            history_text
        ),
        "en" => format!(
            "Here is the conversation history:\n{}\n\n\
             Generate one follow-up question. Output only the question.",
            history_text
        ),
        "ar" => format!(
            "هذا هو سجل المحادثة:\n{}\n\n\
             قم بإنشاء سؤال متابعة واحد. أخرج السؤال فقط.",
            history_text
        ),
        "uk" => format!(
            "Ось історія розмови:\n{}\n\n\
             Згенеруйте одне додаткове запитання. Виведіть лише запитання.",
            history_text
        ),
        "es" => format!(
            "Este es el historial de la conversación:\n{}\n\n\
             Genera una pregunta de seguimiento. Devuelve solo la pregunta.",
            history_text
        ),
        _ => unreachable!(),
    }
}

/// 下書き案 (3 パターン) を生成するシステムプロンプトテンプレート。
/// `{max_chars}` プレースホルダに最大文字数を埋め込む。
pub fn drafts_system(language: &str, max_chars: usize) -> String {
    match normalize_lang(language) {
        "ja" => format!(
            "あなたはジャーナリング支援アシスタントです。ユーザーの入力を基に日記の下書き案を3パターン生成してください。\n\
             各パターンは異なるスタイル（内省的、感謝ベース、事実記録）で作成してください。\n\
             各パターンは{}文字以内にしてください。\n\
             ユーザーが自分の言葉で書き直すための「たたき台」であることを意識してください。\n\
             出力形式：各パターンを「---」で区切ってください。",
            max_chars
        ),
        "en" => format!(
            "You are a journaling assistant. Generate 3 draft patterns based on the user's input.\n\
             Each pattern should use a different style (reflective, gratitude-based, factual).\n\
             Keep each under {} characters.\n\
             Format: separate each pattern with '---'.",
            max_chars
        ),
        "ar" => format!(
            "أنت مساعد لكتابة اليوميات. قم بإنشاء 3 أنماط مسودة بناءً على إدخال المستخدم.\n\
             يجب أن يستخدم كل نمط أسلوباً مختلفاً (تأملي، قائم على الامتنان، واقعي).\n\
             حافظ على كل نمط أقل من {} حرفاً.\n\
             التنسيق: افصل بين كل نمط بـ '---'.",
            max_chars
        ),
        "uk" => format!(
            "Ви — асистент щоденника. Згенеруйте 3 чернетки на основі введення користувача.\n\
             Кожна чернетка має використовувати різний стиль (рефлексивний, на основі вдячності, фактологічний).\n\
             Кожна чернетка має містити менше {} символів.\n\
             Формат: розділіть кожну чернетку через '---'.",
            max_chars
        ),
        "es" => format!(
            "Eres un asistente de diario. Genera 3 borradores basados en la entrada del usuario.\n\
             Cada borrador debe usar un estilo diferente (reflexivo, basado en gratitud, factual).\n\
             Mantén cada uno con menos de {} caracteres.\n\
             Formato: separa cada borrador con '---'.",
            max_chars
        ),
        _ => unreachable!(),
    }
}

/// 翻訳のレビューステータスを表すフラグ。
/// 当該言語のすべてのプロンプトが reviewed=true の場合のみ「監修済み」とみなす。
pub fn is_reviewed(language: &str) -> bool {
    matches!(normalize_lang(language), "ja" | "en")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_supported_languages_have_system_prompt() {
        for lang in &["ja", "en", "ar", "uk", "es"] {
            let prompt = interview_system(lang);
            assert!(!prompt.is_empty(), "system prompt for {} is empty", lang);
            // 4 つのルールすべてに番号があることを期待 (1, 2, 3, 4 を含む)
            for n in &["1", "2", "3", "4"] {
                assert!(
                    prompt.contains(n),
                    "system prompt for {} is missing rule number {}",
                    lang,
                    n
                );
            }
        }
    }

    #[test]
    fn unknown_language_falls_back_to_english() {
        let prompt_xx = interview_system("xx");
        let prompt_en = interview_system("en");
        assert_eq!(prompt_xx, prompt_en);
    }

    #[test]
    fn first_question_prompts_are_distinct_per_language() {
        let ja = first_question_user_prompt("ja");
        let en = first_question_user_prompt("en");
        let ar = first_question_user_prompt("ar");
        // 異なる言語間ではプロンプト本文が異なることを確認 (英語フォールバック検出)
        assert_ne!(ja, en);
        assert_ne!(en, ar);
    }

    #[test]
    fn next_question_includes_history_text() {
        let history = "Q: hello\nA: hi";
        for lang in &["ja", "en", "ar", "uk", "es"] {
            let prompt = next_question_user_prompt(lang, history);
            assert!(
                prompt.contains(history),
                "next question prompt for {} did not include history",
                lang
            );
        }
    }

    #[test]
    fn drafts_system_includes_max_chars() {
        for lang in &["ja", "en", "ar", "uk", "es"] {
            let prompt = drafts_system(lang, 200);
            assert!(
                prompt.contains("200"),
                "drafts system prompt for {} did not include max_chars",
                lang
            );
        }
    }

    #[test]
    fn is_reviewed_flags_match_expected_state() {
        // ja, en は monolithic な monitoring 対象として reviewed=true。
        assert!(is_reviewed("ja"));
        assert!(is_reviewed("en"));
        // ar/uk/es は監修待ち。
        assert!(!is_reviewed("ar"));
        assert!(!is_reviewed("uk"));
        assert!(!is_reviewed("es"));
        // 不明な言語は en に fallback するので reviewed=true 扱い。
        assert!(is_reviewed("xx"));
    }
}
