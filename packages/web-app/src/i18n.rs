//! 国際化 (i18n) ヘルパー。
//!
//! Project Fluent (`fluent-templates::static_loader!`) でビルド時に
//! `locales/{lang}/main.ftl` を埋め込み、実行時に Locale に応じてキーを
//! 解決する。Leptos `view!` の中からは `t!(lang, "key")` で呼ぶ。
//!
//! ## サポート言語
//!
//! 現状: ja, en (Phase 2 マイルストーン v2.4 時点)
//! 将来: ar, uk, es (専門家監修を経て段階的に追加)
//!
//! ## キー命名規則
//!
//! `{page-or-component}-{element-or-purpose}`、すべて kebab-case。
//! 例: `dashboard-title`, `login-flow-not-implemented`。
//!
//! ## 動的引数
//!
//! Fluent 構文 `{ $name }` で位置引数を埋め込める。Rust 側からは
//! `t_args!` マクロで `&[(&str, &FluentValue)]` を渡す。

use fluent_templates::{LanguageIdentifier, Loader, static_loader};
use std::borrow::Cow;
use std::collections::HashMap;
use unic_langid::langid;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        // Fluent の改行扱いを「BiDi 隔離なし」にする (危機文言で見やすくするため)
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

/// プラットフォームがサポートする言語の一覧 (`SUPPORTED_LANGUAGES` と一致させる)。
pub const SUPPORTED_LANGUAGES: &[&str] = &["ja", "en", "ar", "uk", "es"];

/// 翻訳ファイルの実装が揃っている言語の一覧。
/// `SUPPORTED_LANGUAGES` のうち、`locales/{lang}/main.ftl` が存在するもの。
/// 翻訳が間に合わない言語は SUPPORTED に列挙されてもここには含まない。
pub const TRANSLATED_LANGUAGES: &[&str] = &["ja", "en"];

/// デフォルトのフォールバック言語。`Accept-Language` 解決の最終フォールバック。
pub const DEFAULT_LANGUAGE: &str = "en";

/// 言語コード文字列を `LanguageIdentifier` に変換する。
/// 不正な文字列の場合はデフォルト言語を返す。
pub fn parse_lang(lang: &str) -> LanguageIdentifier {
    lang.parse().unwrap_or_else(|_| langid!("en"))
}

/// 翻訳が用意されている言語に正規化する。
/// 用意されていない場合は最も近いフォールバック (英語) を返す。
pub fn normalize_to_translated(lang: &str) -> &'static str {
    for &code in TRANSLATED_LANGUAGES {
        if lang == code {
            return code;
        }
    }
    DEFAULT_LANGUAGE
}

/// 単純なキー → 翻訳文字列の解決。
///
/// `fluent-templates::Loader::lookup` はキーが見つからない場合でもキー名 + エラーマーカ
/// 文字列を返すため、戻り値が String 型 (Option ではない)。
pub fn t(lang: &str, key: &str) -> String {
    let normalized = normalize_to_translated(lang);
    let langid = parse_lang(normalized);
    LOCALES.lookup(&langid, key)
}

/// 引数付きの翻訳解決。Fluent の `{ $name }` 形式を埋める。
///
/// 引数の HashMap のキー型は `Cow<'static, str>` (fluent-templates API 準拠)。
/// 呼び出し側はキー名のリテラル文字列を `Cow::Borrowed` で渡せばよい。
pub fn t_with(
    lang: &str,
    key: &str,
    args: &HashMap<Cow<'static, str>, fluent_templates::fluent_bundle::FluentValue<'_>>,
) -> String {
    let normalized = normalize_to_translated(lang);
    let langid = parse_lang(normalized);
    LOCALES.lookup_with_args(&langid, key, args)
}

/// HTML `dir` 属性 (RTL 対応の準備、現状は ar のみ true)。
pub fn is_rtl(lang: &str) -> bool {
    matches!(lang, "ar" | "he" | "fa" | "ur")
}

/// HTML `dir` 属性値を文字列で返す。
pub fn html_dir(lang: &str) -> &'static str {
    if is_rtl(lang) { "rtl" } else { "ltr" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translated_languages_are_subset_of_supported() {
        for code in TRANSLATED_LANGUAGES {
            assert!(
                SUPPORTED_LANGUAGES.contains(code),
                "translated language {} is not in SUPPORTED_LANGUAGES",
                code
            );
        }
    }

    #[test]
    fn normalize_known_language() {
        assert_eq!(normalize_to_translated("ja"), "ja");
        assert_eq!(normalize_to_translated("en"), "en");
    }

    #[test]
    fn normalize_unknown_language_falls_back() {
        // ar は SUPPORTED だが TRANSLATED ではない (本マイルストーン時点)
        assert_eq!(normalize_to_translated("ar"), "en");
        assert_eq!(normalize_to_translated("xx"), "en");
        assert_eq!(normalize_to_translated(""), "en");
    }

    #[test]
    fn t_resolves_known_key() {
        let result = t("en", "sign-in");
        assert_eq!(result, "Sign in");
    }

    #[test]
    fn t_resolves_japanese() {
        let result = t("ja", "sign-in");
        assert_eq!(result, "サインイン");
    }

    #[test]
    fn t_unknown_key_returns_marker() {
        // fluent-templates の lookup は missing key の場合、エラーマーカ文字列
        // (例 `{key-name}` のような) を返す。完全に空文字列は返さないので、
        // 戻り値が空でないことだけ確認する。
        let result = t("en", "this-key-does-not-exist");
        assert!(!result.is_empty());
    }

    #[test]
    fn t_with_args_substitutes_placeholders() {
        use fluent_templates::fluent_bundle::FluentValue;
        let mut args = HashMap::new();
        args.insert(Cow::Borrowed("word"), FluentValue::from("ERASE"));
        let result = t_with("en", "settings-erase-confirm-label", &args);
        assert!(result.contains("ERASE"));
    }

    #[test]
    fn rtl_detection() {
        assert!(is_rtl("ar"));
        assert!(!is_rtl("ja"));
        assert!(!is_rtl("en"));
    }

    #[test]
    fn html_dir_for_languages() {
        assert_eq!(html_dir("ar"), "rtl");
        assert_eq!(html_dir("ja"), "ltr");
        assert_eq!(html_dir("en"), "ltr");
    }

    #[test]
    fn settings_erase_confirm_word_en() {
        // The English confirmation word must be "ERASE" exactly, so the
        // JS-side comparison is predictable.
        assert_eq!(t("en", "settings-erase-confirm-word"), "ERASE");
    }

    #[test]
    fn settings_erase_confirm_word_ja() {
        assert_eq!(t("ja", "settings-erase-confirm-word"), "消去");
    }

    #[test]
    fn settings_keys_resolve_in_en() {
        for key in &[
            "settings-title",
            "settings-erase-heading",
            "settings-erase-description",
            "settings-erase-button",
            "settings-erase-done",
        ] {
            let result = t("en", key);
            assert!(
                !result.is_empty() && result != *key,
                "key '{}' not resolved in 'en'",
                key
            );
        }
    }
}
