// Many types and methods are public API for the Claude skill integration but not used by CLI/TUI yet
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

// Skill levels for user's knowledge assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLevel {
    Unknown = 0,
    Novice = 1,
    Beginner = 2,
    Intermediate = 3,
    Advanced = 4,
    Expert = 5,
}

impl SkillLevel {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => SkillLevel::Novice,
            2 => SkillLevel::Beginner,
            3 => SkillLevel::Intermediate,
            4 => SkillLevel::Advanced,
            5 => SkillLevel::Expert,
            _ => SkillLevel::Unknown,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SkillLevel::Unknown => "Unknown",
            SkillLevel::Novice => "Novice",
            SkillLevel::Beginner => "Beginner",
            SkillLevel::Intermediate => "Intermediate",
            SkillLevel::Advanced => "Advanced",
            SkillLevel::Expert => "Expert",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "unknown" | "0" => Some(SkillLevel::Unknown),
            "novice" | "1" => Some(SkillLevel::Novice),
            "beginner" | "2" => Some(SkillLevel::Beginner),
            "intermediate" | "3" => Some(SkillLevel::Intermediate),
            "advanced" | "4" => Some(SkillLevel::Advanced),
            "expert" | "5" => Some(SkillLevel::Expert),
            _ => None,
        }
    }
}

// How was the skill level assessed?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssessmentMethod {
    None,
    SelfAssessed,
    Calibration,
}

impl AssessmentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssessmentMethod::None => "none",
            AssessmentMethod::SelfAssessed => "self",
            AssessmentMethod::Calibration => "calibration",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "self" | "self_assessed" | "self-assessed" => AssessmentMethod::SelfAssessed,
            "calibration" | "calibrated" => AssessmentMethod::Calibration,
            _ => AssessmentMethod::None,
        }
    }
}

// Learning session types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionType {
    Feynman,
    Socratic,
}

impl SessionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionType::Feynman => "feynman",
            SessionType::Socratic => "socratic",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "feynman" | "f" => Some(SessionType::Feynman),
            "socratic" | "s" => Some(SessionType::Socratic),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SessionType::Feynman => "User explains topic, Claude identifies gaps",
            SessionType::Socratic => "Claude guides via questions, user discovers answers",
        }
    }
}

// Session outcome (extends ReviewOutcome with abandoned)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionOutcome {
    Success,
    Partial,
    Fail,
    Abandoned,
}

impl SessionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionOutcome::Success => "success",
            SessionOutcome::Partial => "partial",
            SessionOutcome::Fail => "fail",
            SessionOutcome::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "success" | "s" => Some(SessionOutcome::Success),
            "partial" | "p" => Some(SessionOutcome::Partial),
            "fail" | "f" => Some(SessionOutcome::Fail),
            "abandoned" | "a" | "quit" | "q" => Some(SessionOutcome::Abandoned),
            _ => None,
        }
    }

    pub fn to_review_outcome(self) -> Option<ReviewOutcome> {
        match self {
            SessionOutcome::Success => Some(ReviewOutcome::Success),
            SessionOutcome::Partial => Some(ReviewOutcome::Partial),
            SessionOutcome::Fail => Some(ReviewOutcome::Fail),
            SessionOutcome::Abandoned => None,
        }
    }
}

// A full learning session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSession {
    pub id: i64,
    pub topic_id: i64,
    pub session_type: SessionType,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub skill_level_at_start: Option<i32>,
    pub outcome: Option<SessionOutcome>,
    pub summary: Option<String>,
    pub notes: Option<String>,
}

// A knowledge gap identified during a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGap {
    pub id: i64,
    pub session_id: i64,
    pub gap_description: String,
    pub addressed: bool,
}

// A skill assessment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAssessment {
    pub id: i64,
    pub topic_id: i64,
    pub assessed_at: String,
    pub method: AssessmentMethod,
    pub previous_level: Option<i32>,
    pub new_level: i32,
    pub notes: Option<String>,
}

// === Plan/Interview Mode ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Interviewing,
    SpecReady,
    Approved,
    InProgress,
    Complete,
    Abandoned,
}

impl PlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanStatus::Interviewing => "interviewing",
            PlanStatus::SpecReady => "spec_ready",
            PlanStatus::Approved => "approved",
            PlanStatus::InProgress => "in_progress",
            PlanStatus::Complete => "complete",
            PlanStatus::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "interviewing" => Some(PlanStatus::Interviewing),
            "spec_ready" => Some(PlanStatus::SpecReady),
            "approved" => Some(PlanStatus::Approved),
            "in_progress" => Some(PlanStatus::InProgress),
            "complete" => Some(PlanStatus::Complete),
            "abandoned" => Some(PlanStatus::Abandoned),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlanStatus::Interviewing => "Interviewing",
            PlanStatus::SpecReady => "Spec Ready",
            PlanStatus::Approved => "Approved",
            PlanStatus::InProgress => "In Progress",
            PlanStatus::Complete => "Complete",
            PlanStatus::Abandoned => "Abandoned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterviewEntryType {
    Question,
    Answer,
    Note,
    Clarification,
    Decision,
}

impl InterviewEntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterviewEntryType::Question => "question",
            InterviewEntryType::Answer => "answer",
            InterviewEntryType::Note => "note",
            InterviewEntryType::Clarification => "clarification",
            InterviewEntryType::Decision => "decision",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "question" => Some(InterviewEntryType::Question),
            "answer" => Some(InterviewEntryType::Answer),
            "note" => Some(InterviewEntryType::Note),
            "clarification" => Some(InterviewEntryType::Clarification),
            "decision" => Some(InterviewEntryType::Decision),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterviewCategory {
    Requirements,
    EdgeCases,
    Security,
    Deployment,
    Architecture,
    Performance,
    Testing,
    DoD,
    Scope,
    Dependencies,
    Risks,
    Other,
}

impl InterviewCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterviewCategory::Requirements => "requirements",
            InterviewCategory::EdgeCases => "edge_cases",
            InterviewCategory::Security => "security",
            InterviewCategory::Deployment => "deployment",
            InterviewCategory::Architecture => "architecture",
            InterviewCategory::Performance => "performance",
            InterviewCategory::Testing => "testing",
            InterviewCategory::DoD => "dod",
            InterviewCategory::Scope => "scope",
            InterviewCategory::Dependencies => "dependencies",
            InterviewCategory::Risks => "risks",
            InterviewCategory::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "requirements" => Some(InterviewCategory::Requirements),
            "edge_cases" => Some(InterviewCategory::EdgeCases),
            "security" => Some(InterviewCategory::Security),
            "deployment" => Some(InterviewCategory::Deployment),
            "architecture" => Some(InterviewCategory::Architecture),
            "performance" => Some(InterviewCategory::Performance),
            "testing" => Some(InterviewCategory::Testing),
            "dod" => Some(InterviewCategory::DoD),
            "scope" => Some(InterviewCategory::Scope),
            "dependencies" => Some(InterviewCategory::Dependencies),
            "risks" => Some(InterviewCategory::Risks),
            "other" => Some(InterviewCategory::Other),
            _ => Some(InterviewCategory::Other),
        }
    }
}

// A plan/interview record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: i64,
    pub title: String,
    pub initial_description: String,
    pub status: PlanStatus,
    pub engineer_level: Option<String>,
    pub spec_file_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// An entry in the plan interview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewEntry {
    pub id: i64,
    pub plan_id: i64,
    pub entry_type: InterviewEntryType,
    pub content: String,
    pub category: InterviewCategory,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub topic_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub id: i64,
    pub topic_id: i64,
    pub mastery_level: i32,
    pub times_reviewed: i32,
    pub times_succeeded: i32,
    pub last_reviewed: Option<String>,
    pub next_review: Option<String>,
    pub notes: Option<String>,
    // Skill level tracking
    pub skill_level: SkillLevel,
    pub assessment_method: AssessmentMethod,
    pub last_assessed: Option<String>,
}

impl Progress {
    pub fn mastery_label(&self) -> &'static str {
        match self.mastery_level {
            0 => "New",
            1 => "Learning",
            2 => "Familiar",
            3 => "Comfortable",
            4 => "Proficient",
            5 => "Mastered",
            _ => "Unknown",
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.times_reviewed == 0 {
            0.0
        } else {
            (self.times_succeeded as f64 / self.times_reviewed as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicWithProgress {
    pub topic: Topic,
    pub progress: Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewOutcome {
    Success,
    Partial,
    Fail,
}

impl ReviewOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewOutcome::Success => "success",
            ReviewOutcome::Partial => "partial",
            ReviewOutcome::Fail => "fail",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "success" | "s" | "yes" | "y" | "good" | "1" => Some(ReviewOutcome::Success),
            "partial" | "p" | "maybe" | "ok" | "2" => Some(ReviewOutcome::Partial),
            "fail" | "f" | "no" | "n" | "bad" | "0" | "3" => Some(ReviewOutcome::Fail),
            _ => None,
        }
    }
}

// JSON output wrapper for CLI
#[derive(Debug, Serialize)]
pub struct JsonOutput<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> JsonOutput<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod progress_tests {
        use super::*;

        fn make_progress(
            mastery_level: i32,
            times_reviewed: i32,
            times_succeeded: i32,
        ) -> Progress {
            Progress {
                id: 1,
                topic_id: 1,
                mastery_level,
                times_reviewed,
                times_succeeded,
                last_reviewed: None,
                next_review: None,
                notes: None,
                skill_level: SkillLevel::Unknown,
                assessment_method: AssessmentMethod::None,
                last_assessed: None,
            }
        }

        #[test]
        fn mastery_label_level_0() {
            let p = make_progress(0, 0, 0);
            assert_eq!(p.mastery_label(), "New");
        }

        #[test]
        fn mastery_label_level_1() {
            let p = make_progress(1, 1, 1);
            assert_eq!(p.mastery_label(), "Learning");
        }

        #[test]
        fn mastery_label_level_2() {
            let p = make_progress(2, 2, 2);
            assert_eq!(p.mastery_label(), "Familiar");
        }

        #[test]
        fn mastery_label_level_3() {
            let p = make_progress(3, 3, 3);
            assert_eq!(p.mastery_label(), "Comfortable");
        }

        #[test]
        fn mastery_label_level_4() {
            let p = make_progress(4, 4, 4);
            assert_eq!(p.mastery_label(), "Proficient");
        }

        #[test]
        fn mastery_label_level_5() {
            let p = make_progress(5, 5, 5);
            assert_eq!(p.mastery_label(), "Mastered");
        }

        #[test]
        fn mastery_label_invalid_level() {
            let p = make_progress(99, 0, 0);
            assert_eq!(p.mastery_label(), "Unknown");
        }

        #[test]
        fn mastery_label_negative_level() {
            let p = make_progress(-1, 0, 0);
            assert_eq!(p.mastery_label(), "Unknown");
        }

        #[test]
        fn success_rate_zero_reviews() {
            let p = make_progress(0, 0, 0);
            assert_eq!(p.success_rate(), 0.0);
        }

        #[test]
        fn success_rate_all_success() {
            let p = make_progress(0, 10, 10);
            assert_eq!(p.success_rate(), 100.0);
        }

        #[test]
        fn success_rate_half_success() {
            let p = make_progress(0, 10, 5);
            assert_eq!(p.success_rate(), 50.0);
        }

        #[test]
        fn success_rate_no_success() {
            let p = make_progress(0, 10, 0);
            assert_eq!(p.success_rate(), 0.0);
        }

        #[test]
        fn success_rate_partial() {
            let p = make_progress(0, 4, 3);
            assert_eq!(p.success_rate(), 75.0);
        }
    }

    mod review_outcome_tests {
        use super::*;

        #[test]
        fn as_str_success() {
            assert_eq!(ReviewOutcome::Success.as_str(), "success");
        }

        #[test]
        fn as_str_partial() {
            assert_eq!(ReviewOutcome::Partial.as_str(), "partial");
        }

        #[test]
        fn as_str_fail() {
            assert_eq!(ReviewOutcome::Fail.as_str(), "fail");
        }

        #[test]
        fn from_str_success_variants() {
            let variants = ["success", "s", "yes", "y", "good", "1", "SUCCESS", "Yes"];
            for v in variants {
                assert!(
                    matches!(ReviewOutcome::from_str(v), Some(ReviewOutcome::Success)),
                    "Expected Success for '{}'",
                    v
                );
            }
        }

        #[test]
        fn from_str_partial_variants() {
            let variants = ["partial", "p", "maybe", "ok", "2", "PARTIAL", "Maybe"];
            for v in variants {
                assert!(
                    matches!(ReviewOutcome::from_str(v), Some(ReviewOutcome::Partial)),
                    "Expected Partial for '{}'",
                    v
                );
            }
        }

        #[test]
        fn from_str_fail_variants() {
            let variants = ["fail", "f", "no", "n", "bad", "0", "3", "FAIL", "No"];
            for v in variants {
                assert!(
                    matches!(ReviewOutcome::from_str(v), Some(ReviewOutcome::Fail)),
                    "Expected Fail for '{}'",
                    v
                );
            }
        }

        #[test]
        fn from_str_invalid() {
            assert!(ReviewOutcome::from_str("invalid").is_none());
            assert!(ReviewOutcome::from_str("").is_none());
            assert!(ReviewOutcome::from_str("123").is_none());
            assert!(ReviewOutcome::from_str("   ").is_none());
        }
    }

    mod json_output_tests {
        use super::*;

        #[test]
        fn ok_with_string() {
            let output = JsonOutput::ok("test data");
            assert!(output.success);
            assert_eq!(output.data, Some("test data"));
            assert!(output.error.is_none());
        }

        #[test]
        fn ok_with_number() {
            let output = JsonOutput::ok(42);
            assert!(output.success);
            assert_eq!(output.data, Some(42));
            assert!(output.error.is_none());
        }

        #[test]
        fn ok_with_unit() {
            let output = JsonOutput::<()>::ok(());
            assert!(output.success);
            assert_eq!(output.data, Some(()));
            assert!(output.error.is_none());
        }

        #[test]
        fn err_with_string() {
            let output = JsonOutput::<()>::err("something went wrong");
            assert!(!output.success);
            assert!(output.data.is_none());
            assert_eq!(output.error, Some("something went wrong".to_string()));
        }

        #[test]
        fn err_with_owned_string() {
            let output = JsonOutput::<()>::err(String::from("error message"));
            assert!(!output.success);
            assert!(output.data.is_none());
            assert_eq!(output.error, Some("error message".to_string()));
        }

        #[test]
        fn serializes_ok_correctly() {
            let output = JsonOutput::ok("test");
            let json = serde_json::to_string(&output).unwrap();
            assert!(json.contains("\"success\":true"));
            assert!(json.contains("\"data\":\"test\""));
            assert!(json.contains("\"error\":null"));
        }

        #[test]
        fn serializes_err_correctly() {
            let output = JsonOutput::<()>::err("error");
            let json = serde_json::to_string(&output).unwrap();
            assert!(json.contains("\"success\":false"));
            assert!(json.contains("\"data\":null"));
            assert!(json.contains("\"error\":\"error\""));
        }
    }

    mod skill_level_tests {
        use super::*;

        #[test]
        fn as_i32_returns_correct_values() {
            assert_eq!(SkillLevel::Unknown.as_i32(), 0);
            assert_eq!(SkillLevel::Novice.as_i32(), 1);
            assert_eq!(SkillLevel::Beginner.as_i32(), 2);
            assert_eq!(SkillLevel::Intermediate.as_i32(), 3);
            assert_eq!(SkillLevel::Advanced.as_i32(), 4);
            assert_eq!(SkillLevel::Expert.as_i32(), 5);
        }

        #[test]
        fn from_i32_returns_correct_variants() {
            assert_eq!(SkillLevel::from_i32(0), SkillLevel::Unknown);
            assert_eq!(SkillLevel::from_i32(1), SkillLevel::Novice);
            assert_eq!(SkillLevel::from_i32(2), SkillLevel::Beginner);
            assert_eq!(SkillLevel::from_i32(3), SkillLevel::Intermediate);
            assert_eq!(SkillLevel::from_i32(4), SkillLevel::Advanced);
            assert_eq!(SkillLevel::from_i32(5), SkillLevel::Expert);
        }

        #[test]
        fn from_i32_invalid_returns_unknown() {
            assert_eq!(SkillLevel::from_i32(-1), SkillLevel::Unknown);
            assert_eq!(SkillLevel::from_i32(6), SkillLevel::Unknown);
            assert_eq!(SkillLevel::from_i32(100), SkillLevel::Unknown);
        }

        #[test]
        fn label_returns_correct_strings() {
            assert_eq!(SkillLevel::Unknown.label(), "Unknown");
            assert_eq!(SkillLevel::Novice.label(), "Novice");
            assert_eq!(SkillLevel::Beginner.label(), "Beginner");
            assert_eq!(SkillLevel::Intermediate.label(), "Intermediate");
            assert_eq!(SkillLevel::Advanced.label(), "Advanced");
            assert_eq!(SkillLevel::Expert.label(), "Expert");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(SkillLevel::from_str("unknown"), Some(SkillLevel::Unknown));
            assert_eq!(SkillLevel::from_str("novice"), Some(SkillLevel::Novice));
            assert_eq!(SkillLevel::from_str("beginner"), Some(SkillLevel::Beginner));
            assert_eq!(
                SkillLevel::from_str("intermediate"),
                Some(SkillLevel::Intermediate)
            );
            assert_eq!(SkillLevel::from_str("advanced"), Some(SkillLevel::Advanced));
            assert_eq!(SkillLevel::from_str("expert"), Some(SkillLevel::Expert));
        }

        #[test]
        fn from_str_numeric_inputs() {
            assert_eq!(SkillLevel::from_str("0"), Some(SkillLevel::Unknown));
            assert_eq!(SkillLevel::from_str("1"), Some(SkillLevel::Novice));
            assert_eq!(SkillLevel::from_str("5"), Some(SkillLevel::Expert));
        }

        #[test]
        fn from_str_invalid_returns_none() {
            assert_eq!(SkillLevel::from_str("invalid"), None);
            assert_eq!(SkillLevel::from_str(""), None);
        }
    }

    mod session_type_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(SessionType::Feynman.as_str(), "feynman");
            assert_eq!(SessionType::Socratic.as_str(), "socratic");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(SessionType::from_str("feynman"), Some(SessionType::Feynman));
            assert_eq!(SessionType::from_str("f"), Some(SessionType::Feynman));
            assert_eq!(
                SessionType::from_str("socratic"),
                Some(SessionType::Socratic)
            );
            assert_eq!(SessionType::from_str("s"), Some(SessionType::Socratic));
        }

        #[test]
        fn from_str_case_insensitive() {
            assert_eq!(SessionType::from_str("FEYNMAN"), Some(SessionType::Feynman));
            assert_eq!(
                SessionType::from_str("Socratic"),
                Some(SessionType::Socratic)
            );
        }

        #[test]
        fn from_str_invalid_returns_none() {
            assert_eq!(SessionType::from_str("invalid"), None);
            assert_eq!(SessionType::from_str(""), None);
        }

        #[test]
        fn description_returns_meaningful_text() {
            assert!(SessionType::Feynman.description().contains("explains"));
            assert!(SessionType::Socratic.description().contains("questions"));
        }
    }

    mod session_outcome_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(SessionOutcome::Success.as_str(), "success");
            assert_eq!(SessionOutcome::Partial.as_str(), "partial");
            assert_eq!(SessionOutcome::Fail.as_str(), "fail");
            assert_eq!(SessionOutcome::Abandoned.as_str(), "abandoned");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(
                SessionOutcome::from_str("success"),
                Some(SessionOutcome::Success)
            );
            assert_eq!(SessionOutcome::from_str("s"), Some(SessionOutcome::Success));
            assert_eq!(
                SessionOutcome::from_str("partial"),
                Some(SessionOutcome::Partial)
            );
            assert_eq!(SessionOutcome::from_str("fail"), Some(SessionOutcome::Fail));
            assert_eq!(
                SessionOutcome::from_str("abandoned"),
                Some(SessionOutcome::Abandoned)
            );
            assert_eq!(
                SessionOutcome::from_str("quit"),
                Some(SessionOutcome::Abandoned)
            );
        }

        #[test]
        fn to_review_outcome_converts_correctly() {
            assert_eq!(
                SessionOutcome::Success.to_review_outcome(),
                Some(ReviewOutcome::Success)
            );
            assert_eq!(
                SessionOutcome::Partial.to_review_outcome(),
                Some(ReviewOutcome::Partial)
            );
            assert_eq!(
                SessionOutcome::Fail.to_review_outcome(),
                Some(ReviewOutcome::Fail)
            );
            assert_eq!(SessionOutcome::Abandoned.to_review_outcome(), None);
        }
    }

    mod assessment_method_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(AssessmentMethod::None.as_str(), "none");
            assert_eq!(AssessmentMethod::SelfAssessed.as_str(), "self");
            assert_eq!(AssessmentMethod::Calibration.as_str(), "calibration");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(
                AssessmentMethod::from_str("self"),
                AssessmentMethod::SelfAssessed
            );
            assert_eq!(
                AssessmentMethod::from_str("calibration"),
                AssessmentMethod::Calibration
            );
            assert_eq!(AssessmentMethod::from_str("none"), AssessmentMethod::None);
        }

        #[test]
        fn from_str_invalid_returns_none() {
            assert_eq!(
                AssessmentMethod::from_str("invalid"),
                AssessmentMethod::None
            );
        }
    }

    mod plan_status_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(PlanStatus::Interviewing.as_str(), "interviewing");
            assert_eq!(PlanStatus::SpecReady.as_str(), "spec_ready");
            assert_eq!(PlanStatus::Approved.as_str(), "approved");
            assert_eq!(PlanStatus::InProgress.as_str(), "in_progress");
            assert_eq!(PlanStatus::Complete.as_str(), "complete");
            assert_eq!(PlanStatus::Abandoned.as_str(), "abandoned");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(
                PlanStatus::from_str("interviewing"),
                Some(PlanStatus::Interviewing)
            );
            assert_eq!(
                PlanStatus::from_str("spec_ready"),
                Some(PlanStatus::SpecReady)
            );
            assert_eq!(PlanStatus::from_str("complete"), Some(PlanStatus::Complete));
        }

        #[test]
        fn from_str_invalid_returns_none() {
            assert_eq!(PlanStatus::from_str("invalid"), None);
        }

        #[test]
        fn label_returns_human_readable() {
            assert_eq!(PlanStatus::Interviewing.label(), "Interviewing");
            assert_eq!(PlanStatus::SpecReady.label(), "Spec Ready");
            assert_eq!(PlanStatus::InProgress.label(), "In Progress");
        }
    }

    mod interview_entry_type_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(InterviewEntryType::Question.as_str(), "question");
            assert_eq!(InterviewEntryType::Answer.as_str(), "answer");
            assert_eq!(InterviewEntryType::Note.as_str(), "note");
            assert_eq!(InterviewEntryType::Clarification.as_str(), "clarification");
            assert_eq!(InterviewEntryType::Decision.as_str(), "decision");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(
                InterviewEntryType::from_str("question"),
                Some(InterviewEntryType::Question)
            );
            assert_eq!(
                InterviewEntryType::from_str("answer"),
                Some(InterviewEntryType::Answer)
            );
            assert_eq!(
                InterviewEntryType::from_str("decision"),
                Some(InterviewEntryType::Decision)
            );
        }

        #[test]
        fn from_str_invalid_returns_none() {
            assert_eq!(InterviewEntryType::from_str("invalid"), None);
        }
    }

    mod interview_category_tests {
        use super::*;

        #[test]
        fn as_str_returns_correct_values() {
            assert_eq!(InterviewCategory::Requirements.as_str(), "requirements");
            assert_eq!(InterviewCategory::Security.as_str(), "security");
            assert_eq!(InterviewCategory::Architecture.as_str(), "architecture");
            assert_eq!(InterviewCategory::DoD.as_str(), "dod");
        }

        #[test]
        fn from_str_valid_inputs() {
            assert_eq!(
                InterviewCategory::from_str("requirements"),
                Some(InterviewCategory::Requirements)
            );
            assert_eq!(
                InterviewCategory::from_str("edge_cases"),
                Some(InterviewCategory::EdgeCases)
            );
            assert_eq!(
                InterviewCategory::from_str("security"),
                Some(InterviewCategory::Security)
            );
        }

        #[test]
        fn from_str_unknown_returns_other() {
            assert_eq!(
                InterviewCategory::from_str("unknown_category"),
                Some(InterviewCategory::Other)
            );
        }
    }
}
