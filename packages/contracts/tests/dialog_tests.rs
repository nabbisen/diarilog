//! `contracts::dialog` の契約テスト (serde ラウンドトリップ、判別値の安定性)

use contracts::dialog::{AnswerType, QuestionPayload, SessionStatus, StartSessionResponse};

#[test]
fn session_status_roundtrip() {
    let statuses = vec![
        SessionStatus::Active,
        SessionStatus::Completed,
        SessionStatus::Abandoned,
        SessionStatus::CrisisPaused,
    ];
    for status in statuses {
        let s = status.as_str();
        let restored = SessionStatus::from_str(s);
        assert_eq!(status, restored);
    }
}

#[test]
fn session_status_unknown_defaults_active() {
    assert_eq!(
        SessionStatus::from_str("unknown_value"),
        SessionStatus::Active
    );
}

#[test]
fn session_status_serialization_is_snake_case() {
    assert_eq!(
        serde_json::to_string(&SessionStatus::CrisisPaused).unwrap(),
        "\"crisis_paused\""
    );
    assert_eq!(
        serde_json::to_string(&SessionStatus::Active).unwrap(),
        "\"active\""
    );
}

#[test]
fn answer_type_strings() {
    assert_eq!(AnswerType::Free.as_str(), "free");
    assert_eq!(AnswerType::Choice.as_str(), "choice");
    assert_eq!(AnswerType::Scale.as_str(), "scale");
}

#[test]
fn start_session_response_json_roundtrip() {
    let response = StartSessionResponse {
        session_id: "sess-1".into(),
        first_question: QuestionPayload {
            turn_id: "turn-1".into(),
            turn_order: 1,
            question: "How are you feeling?".into(),
            answer_type: AnswerType::Free,
            choices: None,
        },
    };
    let json = serde_json::to_string(&response).unwrap();
    let decoded: StartSessionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.session_id, "sess-1");
    assert_eq!(decoded.first_question.turn_order, 1);
    assert_eq!(decoded.first_question.answer_type, AnswerType::Free);
}
