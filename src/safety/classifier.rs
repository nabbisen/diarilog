//! 危機検知ロジック。
//!
//! 1. 多言語キーワードチェック (即時判定、AI 不要)
//! 2. AI ベースの分類器 (`ai_client::classify`) と組み合わせる
//! 3. 危機判定時に言語別の支援リソースを返す

use contracts::safety::{CrisisResources, HotlineInfo, SafetyLevel};

/// キーワードベースの即時危機判定。
/// AI を呼ばずに早期リターンするための高速チェック。
pub fn keyword_crisis_check(text: &str) -> bool {
    let lower = text.to_lowercase();

    // 日本語
    let ja = [
        "死にたい",
        "自殺",
        "殺したい",
        "消えたい",
        "もう限界",
        "生きていたくない",
        "首を吊",
        "飛び降り",
        "薬を大量",
        "リストカット",
    ];
    // 英語
    let en = [
        "kill myself",
        "want to die",
        "suicide",
        "end my life",
        "no reason to live",
        "better off dead",
        "hurt myself",
        "self harm",
        "overdose",
    ];
    // アラビア語
    let ar = ["أريد أن أموت", "انتحار", "أقتل نفسي"];
    // ウクライナ語
    let uk = ["хочу померти", "самогубство", "покінчити з життям"];
    // スペイン語
    let es = ["quiero morir", "suicidio", "matarme", "no quiero vivir"];

    let all: Vec<&str> = ja
        .iter()
        .chain(en.iter())
        .chain(ar.iter())
        .chain(uk.iter())
        .chain(es.iter())
        .copied()
        .collect();

    all.iter().any(|kw| lower.contains(kw))
}

/// AI 分類器の生の出力 (例: "CRISIS") を SafetyLevel に変換する。
pub fn parse_ai_classification(raw: &str) -> SafetyLevel {
    match raw.trim().to_uppercase().as_str() {
        "CRISIS" => SafetyLevel::Crisis,
        "MILD_CONCERN" => SafetyLevel::MildConcern,
        _ => SafetyLevel::Safe,
    }
}

/// 言語に応じた危機支援リソース。
///
/// ## 翻訳の出典とレビューステータス
///
/// - `reviewed: true` のエントリは、当該言語のメンタルヘルス専門家による
///   監修を受けた翻訳・電話番号・URL を保持する。本番表示してよい。
/// - `reviewed: false` のエントリは暫定値。`docs/i18n-review-flow.md` に従って
///   レビューを依頼するまで本番フラグ越しに公開しないことが望ましい。
///
/// ## レビュー状況 (2026-05 時点)
///
/// | 言語 | message | hotlines | 備考 |
/// |---|---|---|---|
/// | ja | reviewed | 一部 reviewed | 国内主要 3 窓口、phone 番号は公式 URL から確認済 |
/// | en | reviewed | 一部 reviewed | US 中心、英語圏ユーザーには地域 disclaimer が必要 |
/// | ar | not yet | not yet | 地域 (中東 vs 北アフリカ) で適切な窓口が異なる、専門家監修待ち |
/// | uk | not yet | not yet | Lifeline Ukraine が確認済だが完全性は未保証 |
/// | es | not yet | not yet | スペイン本土向け、ラ米諸国向けは別エントリが必要 |
///
/// ## 国際フォールバック
///
/// 不明な言語の場合は IASP (International Association for Suicide Prevention)
/// の国際窓口リストへ誘導する。
pub fn crisis_resources(language: &str) -> CrisisResources {
    match language {
        "ja" => CrisisResources {
            message: "あなたの安全が最も大切です。つらい気持ちを抱えているなら、\
                      専門の相談窓口に連絡してください。あなたは一人ではありません。"
                .to_string(),
            hotlines: vec![
                HotlineInfo {
                    name: "いのちの電話".to_string(),
                    phone: "0120-783-556".to_string(),
                    url: Some("https://www.inochinodenwa.org/".to_string()),
                    language: "ja".to_string(),
                    region: "JP".to_string(),
                    reviewed: true,
                },
                HotlineInfo {
                    name: "よりそいホットライン".to_string(),
                    phone: "0120-279-338".to_string(),
                    url: Some("https://www.since2011.net/yorisoi/".to_string()),
                    language: "ja".to_string(),
                    region: "JP".to_string(),
                    reviewed: true,
                },
                HotlineInfo {
                    name: "こころの健康相談統一ダイヤル".to_string(),
                    phone: "0570-064-556".to_string(),
                    url: None,
                    language: "ja".to_string(),
                    region: "JP".to_string(),
                    reviewed: true,
                },
            ],
            message_reviewed: true,
        },
        "en" => CrisisResources {
            message: "Your safety matters most. If you're struggling, \
                      please reach out to a crisis helpline. You are not alone."
                .to_string(),
            hotlines: vec![
                HotlineInfo {
                    name: "988 Suicide & Crisis Lifeline".to_string(),
                    phone: "988".to_string(),
                    url: Some("https://988lifeline.org/".to_string()),
                    language: "en".to_string(),
                    region: "US".to_string(),
                    reviewed: true,
                },
                HotlineInfo {
                    name: "Crisis Text Line".to_string(),
                    phone: "Text HOME to 741741".to_string(),
                    url: Some("https://www.crisistextline.org/".to_string()),
                    language: "en".to_string(),
                    region: "US".to_string(),
                    reviewed: true,
                },
                HotlineInfo {
                    name: "International Association for Suicide Prevention".to_string(),
                    phone: "See website for local crisis center".to_string(),
                    url: Some("https://www.iasp.info/resources/Crisis_Centres/".to_string()),
                    language: "en".to_string(),
                    region: "International".to_string(),
                    reviewed: true,
                },
            ],
            message_reviewed: true,
        },
        // ──────────────────────────────────────────────────────────────
        // 以下 ar/uk/es は **暫定翻訳**。専門家監修前のため reviewed=false。
        // 監修プロセス完了後に reviewed=true に更新する。
        // ──────────────────────────────────────────────────────────────
        "ar" => CrisisResources {
            message: "سلامتك هي الأهم. إذا كنت تمر بوقت صعب، يرجى التواصل مع خط مساعدة الأزمات. أنت لست وحدك.".to_string(),
            hotlines: vec![
                HotlineInfo {
                    name: "خط نجدة الصحة النفسية (السعودية)".to_string(),
                    phone: "920033360".to_string(),
                    url: None,
                    language: "ar".to_string(),
                    region: "SA".to_string(),
                    reviewed: false,
                },
                HotlineInfo {
                    name: "International Association for Suicide Prevention".to_string(),
                    phone: "See website for local crisis center".to_string(),
                    url: Some("https://www.iasp.info/resources/Crisis_Centres/".to_string()),
                    language: "en".to_string(),
                    region: "International".to_string(),
                    reviewed: true,
                },
            ],
            message_reviewed: false,
        },
        "uk" => CrisisResources {
            message: "Ваша безпека найважливіша. Якщо вам важко, зверніться на гарячу лінію. Ви не самотні.".to_string(),
            hotlines: vec![
                HotlineInfo {
                    name: "Лайфлайн Україна".to_string(),
                    phone: "7333".to_string(),
                    url: Some("https://lifelineukraine.com/".to_string()),
                    language: "uk".to_string(),
                    region: "UA".to_string(),
                    reviewed: false,
                },
                HotlineInfo {
                    name: "International Association for Suicide Prevention".to_string(),
                    phone: "See website for local crisis center".to_string(),
                    url: Some("https://www.iasp.info/resources/Crisis_Centres/".to_string()),
                    language: "en".to_string(),
                    region: "International".to_string(),
                    reviewed: true,
                },
            ],
            message_reviewed: false,
        },
        "es" => CrisisResources {
            message: "Tu seguridad es lo más importante. Si estás pasando por un momento difícil, \
                      contacta una línea de crisis. No estás solo/a."
                .to_string(),
            hotlines: vec![
                HotlineInfo {
                    name: "Teléfono de la Esperanza".to_string(),
                    phone: "717 003 717".to_string(),
                    url: Some("https://www.telefonodelaesperanza.org/".to_string()),
                    language: "es".to_string(),
                    region: "ES".to_string(),
                    reviewed: false,
                },
                HotlineInfo {
                    name: "International Association for Suicide Prevention".to_string(),
                    phone: "See website for local crisis center".to_string(),
                    url: Some("https://www.iasp.info/resources/Crisis_Centres/".to_string()),
                    language: "en".to_string(),
                    region: "International".to_string(),
                    reviewed: true,
                },
            ],
            message_reviewed: false,
        },
        _ => CrisisResources {
            message: "Your safety matters most. Please reach out to a local crisis helpline."
                .to_string(),
            hotlines: vec![HotlineInfo {
                name: "International Association for Suicide Prevention".to_string(),
                phone: "See website for local crisis center".to_string(),
                url: Some("https://www.iasp.info/resources/Crisis_Centres/".to_string()),
                language: "en".to_string(),
                region: "International".to_string(),
                reviewed: true,
            }],
            message_reviewed: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_crisis_check_ja_positive() {
        assert!(keyword_crisis_check("もう死にたい"));
    }

    #[test]
    fn keyword_crisis_check_en_positive() {
        assert!(keyword_crisis_check("I want to die"));
    }

    #[test]
    fn keyword_crisis_check_negative() {
        assert!(!keyword_crisis_check("今日は穏やかな一日でした"));
        assert!(!keyword_crisis_check("Today was a calm day"));
    }

    #[test]
    fn parse_ai_classification_variants() {
        assert_eq!(parse_ai_classification("CRISIS"), SafetyLevel::Crisis);
        assert_eq!(parse_ai_classification(" crisis "), SafetyLevel::Crisis);
        assert_eq!(
            parse_ai_classification("MILD_CONCERN"),
            SafetyLevel::MildConcern
        );
        assert_eq!(parse_ai_classification("SAFE"), SafetyLevel::Safe);
        assert_eq!(parse_ai_classification("garbage"), SafetyLevel::Safe);
    }

    #[test]
    fn crisis_resources_ja_has_three_hotlines() {
        let resources = crisis_resources("ja");
        assert_eq!(resources.hotlines.len(), 3);
        assert!(resources.message.contains("一人ではありません"));
    }

    #[test]
    fn crisis_resources_unknown_lang_falls_back_to_en() {
        let resources = crisis_resources("xx");
        assert!(!resources.hotlines.is_empty());
    }

    #[test]
    fn crisis_resources_all_supported_languages_have_hotlines() {
        // SUPPORTED_LANGUAGES 全件で必ず最低 1 件の hotline が返ることを保証する。
        // これは「ある言語のユーザーに窓口情報が出ない」という事故を防ぐ防壁。
        for lang in &["ja", "en", "ar", "uk", "es"] {
            let resources = crisis_resources(lang);
            assert!(
                !resources.hotlines.is_empty(),
                "language {} returned no hotlines",
                lang
            );
            assert!(
                !resources.message.is_empty(),
                "language {} returned empty message",
                lang
            );
        }
    }

    #[test]
    fn crisis_resources_reviewed_languages_are_marked() {
        // ja と en は専門家監修済み (reviewed: true) となっている前提。
        // この性質が変わった場合 (例: 翻訳が更新された)、テストを意図的に書き直す。
        assert!(
            crisis_resources("ja").message_reviewed,
            "ja message should be marked reviewed"
        );
        assert!(
            crisis_resources("en").message_reviewed,
            "en message should be marked reviewed"
        );
    }

    #[test]
    fn crisis_resources_pending_review_languages() {
        // ar, uk, es は専門家監修待ち (reviewed: false)。
        // 監修完了したら true に切り替えるとともに本テストを更新する。
        for lang in &["ar", "uk", "es"] {
            let resources = crisis_resources(lang);
            assert!(
                !resources.message_reviewed,
                "language {} should currently be marked as not yet reviewed; \
                 update this test once expert review is complete",
                lang
            );
        }
    }

    #[test]
    fn crisis_resources_always_include_international_fallback() {
        // 監修待ち言語のユーザーが暫定値しか得られないという事故を防ぐため、
        // ar/uk/es には必ず IASP の international fallback が含まれるべき。
        for lang in &["ar", "uk", "es"] {
            let resources = crisis_resources(lang);
            let has_intl = resources
                .hotlines
                .iter()
                .any(|h| h.region == "International" && h.reviewed);
            assert!(
                has_intl,
                "language {} should include an international (reviewed) fallback hotline",
                lang
            );
        }
    }
}
