use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use std::path::Path;

use crate::models::{
    AssessmentMethod, InterviewCategory, InterviewEntry, InterviewEntryType, LearningSession,
    Plan, PlanStatus, Progress, ReviewOutcome, SessionGap, SessionOutcome, SessionType,
    SkillAssessment, SkillLevel, Tag, Topic, TopicWithProgress,
};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS topics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS topic_tags (
                topic_id INTEGER NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (topic_id, tag_id),
                FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS progress (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                topic_id INTEGER NOT NULL UNIQUE,
                mastery_level INTEGER NOT NULL DEFAULT 0,
                times_reviewed INTEGER NOT NULL DEFAULT 0,
                times_succeeded INTEGER NOT NULL DEFAULT 0,
                last_reviewed TEXT,
                next_review TEXT NOT NULL DEFAULT (datetime('now')),
                notes TEXT,
                skill_level INTEGER NOT NULL DEFAULT 0,
                assessment_method TEXT NOT NULL DEFAULT 'none',
                last_assessed TEXT,
                FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS review_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                topic_id INTEGER NOT NULL,
                outcome TEXT NOT NULL,
                reviewed_at TEXT NOT NULL DEFAULT (datetime('now')),
                notes TEXT,
                FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
            );

            -- Learning sessions table (Feynman/Socratic sessions)
            CREATE TABLE IF NOT EXISTS learning_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                topic_id INTEGER NOT NULL,
                session_type TEXT NOT NULL CHECK(session_type IN ('feynman', 'socratic')),
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                ended_at TEXT,
                skill_level_at_start INTEGER,
                outcome TEXT CHECK(outcome IN ('success', 'partial', 'fail', 'abandoned')),
                summary TEXT,
                notes TEXT,
                FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
            );

            -- Knowledge gaps identified during sessions
            CREATE TABLE IF NOT EXISTS session_gaps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                gap_description TEXT NOT NULL,
                addressed INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES learning_sessions(id) ON DELETE CASCADE
            );

            -- Skill assessment history
            CREATE TABLE IF NOT EXISTS skill_assessments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                topic_id INTEGER NOT NULL,
                assessed_at TEXT NOT NULL DEFAULT (datetime('now')),
                method TEXT NOT NULL CHECK(method IN ('none', 'self', 'calibration')),
                previous_level INTEGER,
                new_level INTEGER NOT NULL,
                notes TEXT,
                FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE
            );

            -- Plans (interview mode)
            CREATE TABLE IF NOT EXISTS plans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                initial_description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'interviewing' CHECK(status IN ('interviewing', 'spec_ready', 'approved', 'in_progress', 'complete', 'abandoned')),
                engineer_level TEXT,
                spec_file_path TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Plan interview entries
            CREATE TABLE IF NOT EXISTS plan_interview_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                plan_id INTEGER NOT NULL,
                entry_type TEXT NOT NULL CHECK(entry_type IN ('question', 'answer', 'note', 'clarification', 'decision')),
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'other',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_progress_next_review ON progress(next_review);
            CREATE INDEX IF NOT EXISTS idx_progress_mastery ON progress(mastery_level);
            CREATE INDEX IF NOT EXISTS idx_topic_tags_topic ON topic_tags(topic_id);
            CREATE INDEX IF NOT EXISTS idx_topic_tags_tag ON topic_tags(tag_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_topic ON learning_sessions(topic_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_type ON learning_sessions(session_type);
            CREATE INDEX IF NOT EXISTS idx_session_gaps_session ON session_gaps(session_id);
            CREATE INDEX IF NOT EXISTS idx_skill_assessments_topic ON skill_assessments(topic_id);
            CREATE INDEX IF NOT EXISTS idx_plans_status ON plans(status);
            CREATE INDEX IF NOT EXISTS idx_plan_entries_plan ON plan_interview_entries(plan_id);
            CREATE INDEX IF NOT EXISTS idx_plan_entries_category ON plan_interview_entries(category);
            "#,
        )?;

        // Run migrations for existing databases
        self.migrate()?;

        // Create indexes on migrated columns (after migration ensures columns exist)
        self.conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_progress_skill ON progress(skill_level);",
        )?;

        Ok(())
    }

    // Handle schema migrations for existing databases
    fn migrate(&self) -> Result<()> {
        // Check if skill_level column exists in progress table
        let has_skill_level: bool = self
            .conn
            .prepare("SELECT skill_level FROM progress LIMIT 1")
            .is_ok();

        if !has_skill_level {
            // Add new columns to progress table
            self.conn.execute_batch(
                r#"
                ALTER TABLE progress ADD COLUMN skill_level INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE progress ADD COLUMN assessment_method TEXT NOT NULL DEFAULT 'none';
                ALTER TABLE progress ADD COLUMN last_assessed TEXT;
                "#,
            )?;
        }

        Ok(())
    }

    // Topic operations
    pub fn add_topic(&self, name: &str, description: Option<&str>, tags: &[String]) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO topics (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        let topic_id = self.conn.last_insert_rowid();

        // Initialize progress for this topic
        self.conn.execute(
            "INSERT INTO progress (topic_id) VALUES (?1)",
            params![topic_id],
        )?;

        // Add tags
        for tag in tags {
            let tag_id = self.get_or_create_tag(tag)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO topic_tags (topic_id, tag_id) VALUES (?1, ?2)",
                params![topic_id, tag_id],
            )?;
        }

        Ok(topic_id)
    }

    pub fn get_topic(&self, id: i64) -> Result<Option<Topic>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, created_at, updated_at FROM topics WHERE id = ?1")?;

        let topic = stmt.query_row(params![id], |row| {
            Ok(Topic {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                tags: vec![],
            })
        });

        match topic {
            Ok(mut t) => {
                t.tags = self.get_topic_tags(id)?;
                Ok(Some(t))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_topics(&self, tag_filter: Option<&str>) -> Result<Vec<Topic>> {
        let mut topics: Vec<Topic> = if let Some(tag) = tag_filter {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT DISTINCT t.id, t.name, t.description, t.created_at, t.updated_at
                FROM topics t
                JOIN topic_tags tt ON t.id = tt.topic_id
                JOIN tags tg ON tt.tag_id = tg.id
                WHERE tg.name = ?1
                ORDER BY t.name
                "#,
            )?;

            let rows = stmt.query_map(params![tag], |row| {
                Ok(Topic {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    tags: vec![],
                })
            })?;
            rows.collect::<Result<Vec<_>>>()?
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, created_at, updated_at FROM topics ORDER BY name",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(Topic {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    tags: vec![],
                })
            })?;
            rows.collect::<Result<Vec<_>>>()?
        };

        for topic in &mut topics {
            topic.tags = self.get_topic_tags(topic.id)?;
        }

        Ok(topics)
    }

    pub fn delete_topic(&self, id: i64) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM topics WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn update_topic_tags(&self, topic_id: i64, tags: &[String]) -> Result<()> {
        // Remove existing tags
        self.conn.execute(
            "DELETE FROM topic_tags WHERE topic_id = ?1",
            params![topic_id],
        )?;

        // Add new tags
        for tag in tags {
            let tag_id = self.get_or_create_tag(tag)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO topic_tags (topic_id, tag_id) VALUES (?1, ?2)",
                params![topic_id, tag_id],
            )?;
        }

        Ok(())
    }

    // Tag operations
    fn get_or_create_tag(&self, name: &str) -> Result<i64> {
        // Try to get existing tag
        let existing: Result<i64> =
            self.conn
                .query_row("SELECT id FROM tags WHERE name = ?1", params![name], |row| {
                    row.get(0)
                });

        match existing {
            Ok(id) => Ok(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                self.conn
                    .execute("INSERT INTO tags (name) VALUES (?1)", params![name])?;
                Ok(self.conn.last_insert_rowid())
            }
            Err(e) => Err(e),
        }
    }

    fn get_topic_tags(&self, topic_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT tg.name
            FROM tags tg
            JOIN topic_tags tt ON tg.id = tt.tag_id
            WHERE tt.topic_id = ?1
            ORDER BY tg.name
            "#,
        )?;

        let rows = stmt.query_map(params![topic_id], |row| row.get(0))?;
        let tags = rows.collect::<Result<Vec<String>>>()?;

        Ok(tags)
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT tg.id, tg.name, COUNT(tt.topic_id) as topic_count
            FROM tags tg
            LEFT JOIN topic_tags tt ON tg.id = tt.tag_id
            GROUP BY tg.id, tg.name
            ORDER BY tg.name
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                topic_count: row.get(2)?,
            })
        })?;
        let tags = rows.collect::<Result<Vec<_>>>()?;

        Ok(tags)
    }

    // Progress operations
    pub fn get_progress(&self, topic_id: i64) -> Result<Option<Progress>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, topic_id, mastery_level, times_reviewed, times_succeeded,
                   last_reviewed, next_review, notes, skill_level, assessment_method, last_assessed
            FROM progress
            WHERE topic_id = ?1
            "#,
        )?;

        let progress = stmt.query_row(params![topic_id], |row| {
            let skill_level_int: i32 = row.get(8)?;
            let assessment_str: String = row.get(9)?;
            Ok(Progress {
                id: row.get(0)?,
                topic_id: row.get(1)?,
                mastery_level: row.get(2)?,
                times_reviewed: row.get(3)?,
                times_succeeded: row.get(4)?,
                last_reviewed: row.get(5)?,
                next_review: row.get(6)?,
                notes: row.get(7)?,
                skill_level: SkillLevel::from_i32(skill_level_int),
                assessment_method: AssessmentMethod::from_str(&assessment_str),
                last_assessed: row.get(10)?,
            })
        });

        match progress {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn record_review(
        &self,
        topic_id: i64,
        outcome: ReviewOutcome,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        let outcome_str = outcome.as_str();

        // Record in history
        self.conn.execute(
            "INSERT INTO review_history (topic_id, outcome, reviewed_at, notes) VALUES (?1, ?2, ?3, ?4)",
            params![topic_id, outcome_str, now.to_rfc3339(), notes],
        )?;

        // Get current progress
        let progress = self.get_progress(topic_id)?.ok_or_else(|| {
            rusqlite::Error::QueryReturnedNoRows
        })?;

        // Calculate new mastery level and next review
        let (new_mastery, days_until_next) = match outcome {
            ReviewOutcome::Success => {
                let new_level = (progress.mastery_level + 1).min(5);
                let days = Self::calculate_interval(new_level);
                (new_level, days)
            }
            ReviewOutcome::Partial => {
                let new_level = progress.mastery_level; // Stay same
                let days = Self::calculate_interval(new_level) / 2;
                (new_level, days.max(1))
            }
            ReviewOutcome::Fail => {
                let new_level = (progress.mastery_level - 1).max(0);
                (new_level, 1) // Review again tomorrow
            }
        };

        let next_review = now + chrono::Duration::days(days_until_next as i64);
        let times_succeeded = if matches!(outcome, ReviewOutcome::Success) {
            progress.times_succeeded + 1
        } else {
            progress.times_succeeded
        };

        self.conn.execute(
            r#"
            UPDATE progress
            SET mastery_level = ?1,
                times_reviewed = times_reviewed + 1,
                times_succeeded = ?2,
                last_reviewed = ?3,
                next_review = ?4,
                notes = COALESCE(?5, notes)
            WHERE topic_id = ?6
            "#,
            params![
                new_mastery,
                times_succeeded,
                now.to_rfc3339(),
                next_review.to_rfc3339(),
                notes,
                topic_id
            ],
        )?;

        Ok(())
    }

    // Spaced repetition intervals (in days) based on mastery level
    fn calculate_interval(mastery_level: i32) -> i32 {
        match mastery_level {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 7,
            4 => 14,
            5 => 30,
            _ => 30,
        }
    }

    // Stochastic selection for next topic to review
    pub fn get_next_topic(&self, tag_filter: Option<&str>) -> Result<Option<TopicWithProgress>> {
        // Get topics due for review, weighted by priority
        let topics = self.get_due_topics(tag_filter)?;

        if topics.is_empty() {
            return Ok(None);
        }

        // Stochastic selection: weight by overdue-ness and lower mastery
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let now = Utc::now();
        let weights: Vec<f64> = topics
            .iter()
            .map(|t| {
                let overdue_days = if let Some(next) = &t.progress.next_review {
                    if let Ok(next_dt) = DateTime::parse_from_rfc3339(next) {
                        let diff = now.signed_duration_since(next_dt.with_timezone(&Utc));
                        diff.num_days().max(0) as f64 + 1.0
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };

                // Lower mastery = higher weight, overdue = higher weight
                let mastery_weight = 6.0 - t.progress.mastery_level as f64;
                overdue_days * mastery_weight
            })
            .collect();

        let total_weight: f64 = weights.iter().sum();
        let mut random_point = rng.gen::<f64>() * total_weight;

        for (i, weight) in weights.iter().enumerate() {
            random_point -= weight;
            if random_point <= 0.0 {
                return Ok(Some(topics[i].clone()));
            }
        }

        // Fallback to first
        Ok(topics.into_iter().next())
    }

    fn get_due_topics(&self, tag_filter: Option<&str>) -> Result<Vec<TopicWithProgress>> {
        let base_query = r#"
            SELECT t.id, t.name, t.description, t.created_at, t.updated_at,
                   p.id, p.topic_id, p.mastery_level, p.times_reviewed, p.times_succeeded,
                   p.last_reviewed, p.next_review, p.notes, p.skill_level, p.assessment_method, p.last_assessed
            FROM topics t
            JOIN progress p ON t.id = p.topic_id
        "#;

        let (query, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(tag) = tag_filter {
            let q = format!(
                r#"{}
                JOIN topic_tags tt ON t.id = tt.topic_id
                JOIN tags tg ON tt.tag_id = tg.id
                WHERE tg.name = ?1
                ORDER BY p.next_review ASC, p.mastery_level ASC
                "#,
                base_query
            );
            (q, vec![Box::new(tag.to_string())])
        } else {
            let q = format!(
                "{} ORDER BY p.next_review ASC, p.mastery_level ASC",
                base_query
            );
            (q, vec![])
        };

        let mut stmt = self.conn.prepare(&query)?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let skill_level_int: i32 = row.get(13)?;
            let assessment_str: String = row.get(14)?;
            Ok(TopicWithProgress {
                topic: Topic {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    tags: vec![],
                },
                progress: Progress {
                    id: row.get(5)?,
                    topic_id: row.get(6)?,
                    mastery_level: row.get(7)?,
                    times_reviewed: row.get(8)?,
                    times_succeeded: row.get(9)?,
                    last_reviewed: row.get(10)?,
                    next_review: row.get(11)?,
                    notes: row.get(12)?,
                    skill_level: SkillLevel::from_i32(skill_level_int),
                    assessment_method: AssessmentMethod::from_str(&assessment_str),
                    last_assessed: row.get(15)?,
                },
            })
        })?;
        let topics = rows.collect::<Result<Vec<_>>>()?;

        // Fill in tags
        let mut result = topics;
        for twp in &mut result {
            twp.topic.tags = self.get_topic_tags(twp.topic.id)?;
        }

        Ok(result)
    }

    // Learning session operations
    pub fn start_session(&self, topic_id: i64, session_type: SessionType) -> Result<i64> {
        let now = Utc::now();
        let progress = self.get_progress(topic_id)?;
        let skill_at_start = progress.map(|p| p.skill_level.as_i32());

        self.conn.execute(
            r#"
            INSERT INTO learning_sessions (topic_id, session_type, started_at, skill_level_at_start)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![topic_id, session_type.as_str(), now.to_rfc3339(), skill_at_start],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn end_session(
        &self,
        session_id: i64,
        outcome: SessionOutcome,
        summary: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            r#"
            UPDATE learning_sessions
            SET ended_at = ?1, outcome = ?2, summary = ?3, notes = ?4
            WHERE id = ?5
            "#,
            params![now.to_rfc3339(), outcome.as_str(), summary, notes, session_id],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: i64) -> Result<Option<LearningSession>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, topic_id, session_type, started_at, ended_at,
                   skill_level_at_start, outcome, summary, notes
            FROM learning_sessions
            WHERE id = ?1
            "#,
        )?;

        let session = stmt.query_row(params![session_id], |row| {
            let session_type_str: String = row.get(2)?;
            let outcome_str: Option<String> = row.get(6)?;
            Ok(LearningSession {
                id: row.get(0)?,
                topic_id: row.get(1)?,
                session_type: SessionType::from_str(&session_type_str).unwrap_or(SessionType::Feynman),
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                skill_level_at_start: row.get(5)?,
                outcome: outcome_str.and_then(|s| SessionOutcome::from_str(&s)),
                summary: row.get(7)?,
                notes: row.get(8)?,
            })
        });

        match session {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_sessions(&self, topic_id: Option<i64>) -> Result<Vec<LearningSession>> {
        let (query, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(tid) = topic_id {
            (
                r#"
                SELECT id, topic_id, session_type, started_at, ended_at,
                       skill_level_at_start, outcome, summary, notes
                FROM learning_sessions
                WHERE topic_id = ?1
                ORDER BY started_at DESC
                "#.to_string(),
                vec![Box::new(tid)],
            )
        } else {
            (
                r#"
                SELECT id, topic_id, session_type, started_at, ended_at,
                       skill_level_at_start, outcome, summary, notes
                FROM learning_sessions
                ORDER BY started_at DESC
                "#.to_string(),
                vec![],
            )
        };

        let mut stmt = self.conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let session_type_str: String = row.get(2)?;
            let outcome_str: Option<String> = row.get(6)?;
            Ok(LearningSession {
                id: row.get(0)?,
                topic_id: row.get(1)?,
                session_type: SessionType::from_str(&session_type_str).unwrap_or(SessionType::Feynman),
                started_at: row.get(3)?,
                ended_at: row.get(4)?,
                skill_level_at_start: row.get(5)?,
                outcome: outcome_str.and_then(|s| SessionOutcome::from_str(&s)),
                summary: row.get(7)?,
                notes: row.get(8)?,
            })
        })?;

        rows.collect()
    }

    // Session gap operations
    pub fn add_session_gap(&self, session_id: i64, gap_description: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO session_gaps (session_id, gap_description) VALUES (?1, ?2)",
            params![session_id, gap_description],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn mark_gap_addressed(&self, gap_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE session_gaps SET addressed = 1 WHERE id = ?1",
            params![gap_id],
        )?;
        Ok(())
    }

    pub fn get_session_gaps(&self, session_id: i64) -> Result<Vec<SessionGap>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, gap_description, addressed FROM session_gaps WHERE session_id = ?1",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(SessionGap {
                id: row.get(0)?,
                session_id: row.get(1)?,
                gap_description: row.get(2)?,
                addressed: row.get::<_, i32>(3)? != 0,
            })
        })?;

        rows.collect()
    }

    pub fn get_unaddressed_gaps(&self, topic_id: i64) -> Result<Vec<SessionGap>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT sg.id, sg.session_id, sg.gap_description, sg.addressed
            FROM session_gaps sg
            JOIN learning_sessions ls ON sg.session_id = ls.id
            WHERE ls.topic_id = ?1 AND sg.addressed = 0
            ORDER BY ls.started_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![topic_id], |row| {
            Ok(SessionGap {
                id: row.get(0)?,
                session_id: row.get(1)?,
                gap_description: row.get(2)?,
                addressed: row.get::<_, i32>(3)? != 0,
            })
        })?;

        rows.collect()
    }

    // Skill assessment operations
    pub fn update_skill_level(
        &self,
        topic_id: i64,
        new_level: SkillLevel,
        method: AssessmentMethod,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();

        // Get current level for history
        let current = self.get_progress(topic_id)?;
        let previous_level = current.map(|p| p.skill_level.as_i32());

        // Record assessment history
        self.conn.execute(
            r#"
            INSERT INTO skill_assessments (topic_id, assessed_at, method, previous_level, new_level, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                topic_id,
                now.to_rfc3339(),
                method.as_str(),
                previous_level,
                new_level.as_i32(),
                notes
            ],
        )?;

        // Update progress table
        self.conn.execute(
            r#"
            UPDATE progress
            SET skill_level = ?1, assessment_method = ?2, last_assessed = ?3
            WHERE topic_id = ?4
            "#,
            params![new_level.as_i32(), method.as_str(), now.to_rfc3339(), topic_id],
        )?;

        Ok(())
    }

    pub fn get_skill_assessments(&self, topic_id: i64) -> Result<Vec<SkillAssessment>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, topic_id, assessed_at, method, previous_level, new_level, notes
            FROM skill_assessments
            WHERE topic_id = ?1
            ORDER BY assessed_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![topic_id], |row| {
            let method_str: String = row.get(3)?;
            Ok(SkillAssessment {
                id: row.get(0)?,
                topic_id: row.get(1)?,
                assessed_at: row.get(2)?,
                method: AssessmentMethod::from_str(&method_str),
                previous_level: row.get(4)?,
                new_level: row.get(5)?,
                notes: row.get(6)?,
            })
        })?;

        rows.collect()
    }

    // Plan operations
    pub fn create_plan(&self, title: &str, initial_description: &str) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            r#"
            INSERT INTO plans (title, initial_description, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?3)
            "#,
            params![title, initial_description, now.to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_plan(&self, plan_id: i64) -> Result<Option<Plan>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, initial_description, status, engineer_level, spec_file_path, created_at, updated_at
            FROM plans
            WHERE id = ?1
            "#,
        )?;

        let plan = stmt.query_row(params![plan_id], |row| {
            let status_str: String = row.get(3)?;
            Ok(Plan {
                id: row.get(0)?,
                title: row.get(1)?,
                initial_description: row.get(2)?,
                status: PlanStatus::from_str(&status_str).unwrap_or(PlanStatus::Interviewing),
                engineer_level: row.get(4)?,
                spec_file_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        });

        match plan {
            Ok(p) => Ok(Some(p)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_plans(&self, status_filter: Option<PlanStatus>) -> Result<Vec<Plan>> {
        let (query, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(status) = status_filter {
            (
                r#"
                SELECT id, title, initial_description, status, engineer_level, spec_file_path, created_at, updated_at
                FROM plans
                WHERE status = ?1
                ORDER BY updated_at DESC
                "#.to_string(),
                vec![Box::new(status.as_str().to_string())],
            )
        } else {
            (
                r#"
                SELECT id, title, initial_description, status, engineer_level, spec_file_path, created_at, updated_at
                FROM plans
                ORDER BY updated_at DESC
                "#.to_string(),
                vec![],
            )
        };

        let mut stmt = self.conn.prepare(&query)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let status_str: String = row.get(3)?;
            Ok(Plan {
                id: row.get(0)?,
                title: row.get(1)?,
                initial_description: row.get(2)?,
                status: PlanStatus::from_str(&status_str).unwrap_or(PlanStatus::Interviewing),
                engineer_level: row.get(4)?,
                spec_file_path: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        rows.collect()
    }

    pub fn update_plan_status(&self, plan_id: i64, status: PlanStatus) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE plans SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now.to_rfc3339(), plan_id],
        )?;
        Ok(())
    }

    pub fn update_plan_engineer_level(&self, plan_id: i64, level: &str) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE plans SET engineer_level = ?1, updated_at = ?2 WHERE id = ?3",
            params![level, now.to_rfc3339(), plan_id],
        )?;
        Ok(())
    }

    pub fn update_plan_spec_path(&self, plan_id: i64, path: &str) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE plans SET spec_file_path = ?1, status = 'spec_ready', updated_at = ?2 WHERE id = ?3",
            params![path, now.to_rfc3339(), plan_id],
        )?;
        Ok(())
    }

    pub fn delete_plan(&self, plan_id: i64) -> Result<bool> {
        let rows = self.conn.execute("DELETE FROM plans WHERE id = ?1", params![plan_id])?;
        Ok(rows > 0)
    }

    // Plan interview entry operations
    pub fn add_interview_entry(
        &self,
        plan_id: i64,
        entry_type: InterviewEntryType,
        content: &str,
        category: InterviewCategory,
    ) -> Result<i64> {
        let now = Utc::now();
        self.conn.execute(
            r#"
            INSERT INTO plan_interview_entries (plan_id, entry_type, content, category, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![plan_id, entry_type.as_str(), content, category.as_str(), now.to_rfc3339()],
        )?;

        // Update plan's updated_at
        self.conn.execute(
            "UPDATE plans SET updated_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), plan_id],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_interview_entries(&self, plan_id: i64) -> Result<Vec<InterviewEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, plan_id, entry_type, content, category, created_at
            FROM plan_interview_entries
            WHERE plan_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![plan_id], |row| {
            let entry_type_str: String = row.get(2)?;
            let category_str: String = row.get(4)?;
            Ok(InterviewEntry {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                entry_type: InterviewEntryType::from_str(&entry_type_str).unwrap_or(InterviewEntryType::Note),
                content: row.get(3)?,
                category: InterviewCategory::from_str(&category_str).unwrap_or(InterviewCategory::Other),
                created_at: row.get(5)?,
            })
        })?;

        rows.collect()
    }

    pub fn get_interview_entries_by_category(&self, plan_id: i64, category: InterviewCategory) -> Result<Vec<InterviewEntry>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, plan_id, entry_type, content, category, created_at
            FROM plan_interview_entries
            WHERE plan_id = ?1 AND category = ?2
            ORDER BY created_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![plan_id, category.as_str()], |row| {
            let entry_type_str: String = row.get(2)?;
            let category_str: String = row.get(4)?;
            Ok(InterviewEntry {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                entry_type: InterviewEntryType::from_str(&entry_type_str).unwrap_or(InterviewEntryType::Note),
                content: row.get(3)?,
                category: InterviewCategory::from_str(&category_str).unwrap_or(InterviewCategory::Other),
                created_at: row.get(5)?,
            })
        })?;

        rows.collect()
    }

    pub fn get_stats(&self) -> Result<Stats> {
        let total_topics: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM topics", [], |row| row.get(0))?;

        let total_reviews: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM review_history", [], |row| row.get(0))?;

        let mastered: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM progress WHERE mastery_level >= 4",
            [],
            |row| row.get(0),
        )?;

        let due_now: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM progress WHERE next_review <= datetime('now')",
            [],
            |row| row.get(0),
        )?;

        let avg_mastery: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(AVG(mastery_level), 0) FROM progress",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        Ok(Stats {
            total_topics,
            total_reviews,
            mastered,
            due_now,
            avg_mastery,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_topics: i64,
    pub total_reviews: i64,
    pub mastered: i64,
    pub due_now: i64,
    pub avg_mastery: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open(":memory:").expect("Failed to create in-memory database");
        db.init().expect("Failed to initialize database");
        db
    }

    mod init_tests {
        use super::*;

        #[test]
        fn init_creates_tables() {
            let db = setup_db();
            // Verify tables exist by querying them
            let topics: i64 = db
                .conn
                .query_row("SELECT COUNT(*) FROM topics", [], |row| row.get(0))
                .expect("topics table should exist");
            assert_eq!(topics, 0);

            let tags: i64 = db
                .conn
                .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))
                .expect("tags table should exist");
            assert_eq!(tags, 0);

            let progress: i64 = db
                .conn
                .query_row("SELECT COUNT(*) FROM progress", [], |row| row.get(0))
                .expect("progress table should exist");
            assert_eq!(progress, 0);

            let history: i64 = db
                .conn
                .query_row("SELECT COUNT(*) FROM review_history", [], |row| row.get(0))
                .expect("review_history table should exist");
            assert_eq!(history, 0);
        }

        #[test]
        fn init_is_idempotent() {
            let db = setup_db();
            // Add some data
            db.add_topic("Test", None, &[]).unwrap();

            // Re-init should not fail or clear data
            db.init().expect("Re-init should succeed");

            let topics = db.list_topics(None).unwrap();
            assert_eq!(topics.len(), 1);
        }
    }

    mod topic_tests {
        use super::*;

        #[test]
        fn add_topic_basic() {
            let db = setup_db();
            let id = db.add_topic("Rust Basics", None, &[]).unwrap();
            assert!(id > 0);

            let topic = db.get_topic(id).unwrap().unwrap();
            assert_eq!(topic.name, "Rust Basics");
            assert!(topic.description.is_none());
            assert!(topic.tags.is_empty());
        }

        #[test]
        fn add_topic_with_description() {
            let db = setup_db();
            let id = db
                .add_topic("Rust Basics", Some("Learn the fundamentals"), &[])
                .unwrap();

            let topic = db.get_topic(id).unwrap().unwrap();
            assert_eq!(topic.description, Some("Learn the fundamentals".to_string()));
        }

        #[test]
        fn add_topic_with_tags() {
            let db = setup_db();
            let tags = vec!["rust".to_string(), "programming".to_string()];
            let id = db.add_topic("Rust Basics", None, &tags).unwrap();

            let topic = db.get_topic(id).unwrap().unwrap();
            assert_eq!(topic.tags.len(), 2);
            assert!(topic.tags.contains(&"rust".to_string()));
            assert!(topic.tags.contains(&"programming".to_string()));
        }

        #[test]
        fn add_topic_creates_progress() {
            let db = setup_db();
            let id = db.add_topic("Test Topic", None, &[]).unwrap();

            let progress = db.get_progress(id).unwrap().unwrap();
            assert_eq!(progress.topic_id, id);
            assert_eq!(progress.mastery_level, 0);
            assert_eq!(progress.times_reviewed, 0);
            assert_eq!(progress.times_succeeded, 0);
            assert_eq!(progress.skill_level, SkillLevel::Unknown);
            assert_eq!(progress.assessment_method, AssessmentMethod::None);
            assert!(progress.last_assessed.is_none());
        }

        #[test]
        fn add_topic_duplicate_name_fails() {
            let db = setup_db();
            db.add_topic("Unique Name", None, &[]).unwrap();
            let result = db.add_topic("Unique Name", None, &[]);
            assert!(result.is_err());
        }

        #[test]
        fn get_topic_not_found() {
            let db = setup_db();
            let topic = db.get_topic(999).unwrap();
            assert!(topic.is_none());
        }

        #[test]
        fn list_topics_empty() {
            let db = setup_db();
            let topics = db.list_topics(None).unwrap();
            assert!(topics.is_empty());
        }

        #[test]
        fn list_topics_returns_all() {
            let db = setup_db();
            db.add_topic("Topic A", None, &[]).unwrap();
            db.add_topic("Topic B", None, &[]).unwrap();
            db.add_topic("Topic C", None, &[]).unwrap();

            let topics = db.list_topics(None).unwrap();
            assert_eq!(topics.len(), 3);
        }

        #[test]
        fn list_topics_sorted_by_name() {
            let db = setup_db();
            db.add_topic("Zebra", None, &[]).unwrap();
            db.add_topic("Alpha", None, &[]).unwrap();
            db.add_topic("Middle", None, &[]).unwrap();

            let topics = db.list_topics(None).unwrap();
            assert_eq!(topics[0].name, "Alpha");
            assert_eq!(topics[1].name, "Middle");
            assert_eq!(topics[2].name, "Zebra");
        }

        #[test]
        fn list_topics_filter_by_tag() {
            let db = setup_db();
            db.add_topic("Rust Topic", None, &["rust".to_string()])
                .unwrap();
            db.add_topic("Go Topic", None, &["go".to_string()]).unwrap();
            db.add_topic("Both", None, &["rust".to_string(), "go".to_string()])
                .unwrap();

            let rust_topics = db.list_topics(Some("rust")).unwrap();
            assert_eq!(rust_topics.len(), 2);

            let go_topics = db.list_topics(Some("go")).unwrap();
            assert_eq!(go_topics.len(), 2);

            let python_topics = db.list_topics(Some("python")).unwrap();
            assert!(python_topics.is_empty());
        }

        #[test]
        fn delete_topic_success() {
            let db = setup_db();
            let id = db.add_topic("To Delete", None, &[]).unwrap();

            let deleted = db.delete_topic(id).unwrap();
            assert!(deleted);

            let topic = db.get_topic(id).unwrap();
            assert!(topic.is_none());
        }

        #[test]
        fn delete_topic_not_found() {
            let db = setup_db();
            let deleted = db.delete_topic(999).unwrap();
            assert!(!deleted);
        }

        #[test]
        fn delete_topic_cascades_progress() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Verify progress exists
            assert!(db.get_progress(id).unwrap().is_some());

            db.delete_topic(id).unwrap();

            // Progress should be gone too
            assert!(db.get_progress(id).unwrap().is_none());
        }

        #[test]
        fn update_topic_tags() {
            let db = setup_db();
            let id = db
                .add_topic("Test", None, &["old".to_string()])
                .unwrap();

            db.update_topic_tags(id, &["new1".to_string(), "new2".to_string()])
                .unwrap();

            let topic = db.get_topic(id).unwrap().unwrap();
            assert_eq!(topic.tags.len(), 2);
            assert!(topic.tags.contains(&"new1".to_string()));
            assert!(topic.tags.contains(&"new2".to_string()));
            assert!(!topic.tags.contains(&"old".to_string()));
        }

        #[test]
        fn update_topic_tags_to_empty() {
            let db = setup_db();
            let id = db
                .add_topic("Test", None, &["tag1".to_string()])
                .unwrap();

            db.update_topic_tags(id, &[]).unwrap();

            let topic = db.get_topic(id).unwrap().unwrap();
            assert!(topic.tags.is_empty());
        }
    }

    mod tag_tests {
        use super::*;

        #[test]
        fn list_tags_empty() {
            let db = setup_db();
            let tags = db.list_tags().unwrap();
            assert!(tags.is_empty());
        }

        #[test]
        fn list_tags_with_counts() {
            let db = setup_db();
            db.add_topic("T1", None, &["common".to_string()]).unwrap();
            db.add_topic("T2", None, &["common".to_string()]).unwrap();
            db.add_topic("T3", None, &["rare".to_string()]).unwrap();

            let tags = db.list_tags().unwrap();
            assert_eq!(tags.len(), 2);

            let common = tags.iter().find(|t| t.name == "common").unwrap();
            assert_eq!(common.topic_count, 2);

            let rare = tags.iter().find(|t| t.name == "rare").unwrap();
            assert_eq!(rare.topic_count, 1);
        }

        #[test]
        fn tags_are_reused() {
            let db = setup_db();
            db.add_topic("T1", None, &["shared".to_string()]).unwrap();
            db.add_topic("T2", None, &["shared".to_string()]).unwrap();

            let tags = db.list_tags().unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].topic_count, 2);
        }
    }

    mod review_tests {
        use super::*;

        #[test]
        fn record_review_success_increases_mastery() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Initial mastery is 0
            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 0);

            db.record_review(id, ReviewOutcome::Success, None).unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 1);
            assert_eq!(p.times_reviewed, 1);
            assert_eq!(p.times_succeeded, 1);
        }

        #[test]
        fn record_review_success_caps_at_5() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Do 10 successful reviews
            for _ in 0..10 {
                db.record_review(id, ReviewOutcome::Success, None).unwrap();
            }

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 5); // Should cap at 5
            assert_eq!(p.times_reviewed, 10);
            assert_eq!(p.times_succeeded, 10);
        }

        #[test]
        fn record_review_partial_maintains_level() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Get to level 2
            db.record_review(id, ReviewOutcome::Success, None).unwrap();
            db.record_review(id, ReviewOutcome::Success, None).unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 2);

            // Partial should maintain level
            db.record_review(id, ReviewOutcome::Partial, None).unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 2);
            assert_eq!(p.times_reviewed, 3);
            assert_eq!(p.times_succeeded, 2); // Only successes count
        }

        #[test]
        fn record_review_fail_decreases_mastery() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Get to level 3
            for _ in 0..3 {
                db.record_review(id, ReviewOutcome::Success, None).unwrap();
            }

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 3);

            // Fail should decrease
            db.record_review(id, ReviewOutcome::Fail, None).unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 2);
        }

        #[test]
        fn record_review_fail_floors_at_0() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            // Fail multiple times at level 0
            db.record_review(id, ReviewOutcome::Fail, None).unwrap();
            db.record_review(id, ReviewOutcome::Fail, None).unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.mastery_level, 0); // Should not go negative
        }

        #[test]
        fn record_review_with_notes() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            db.record_review(id, ReviewOutcome::Success, Some("Great session!"))
                .unwrap();

            let p = db.get_progress(id).unwrap().unwrap();
            assert_eq!(p.notes, Some("Great session!".to_string()));
        }

        #[test]
        fn record_review_updates_timestamps() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            let p_before = db.get_progress(id).unwrap().unwrap();
            assert!(p_before.last_reviewed.is_none());

            db.record_review(id, ReviewOutcome::Success, None).unwrap();

            let p_after = db.get_progress(id).unwrap().unwrap();
            assert!(p_after.last_reviewed.is_some());
            assert!(p_after.next_review.is_some());
        }

        #[test]
        fn record_review_creates_history() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();

            db.record_review(id, ReviewOutcome::Success, Some("note1"))
                .unwrap();
            db.record_review(id, ReviewOutcome::Fail, Some("note2"))
                .unwrap();

            let count: i64 = db
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM review_history WHERE topic_id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 2);
        }
    }

    mod interval_tests {
        use super::*;

        #[test]
        fn calculate_interval_level_0() {
            assert_eq!(Database::calculate_interval(0), 1);
        }

        #[test]
        fn calculate_interval_level_1() {
            assert_eq!(Database::calculate_interval(1), 2);
        }

        #[test]
        fn calculate_interval_level_2() {
            assert_eq!(Database::calculate_interval(2), 4);
        }

        #[test]
        fn calculate_interval_level_3() {
            assert_eq!(Database::calculate_interval(3), 7);
        }

        #[test]
        fn calculate_interval_level_4() {
            assert_eq!(Database::calculate_interval(4), 14);
        }

        #[test]
        fn calculate_interval_level_5() {
            assert_eq!(Database::calculate_interval(5), 30);
        }

        #[test]
        fn calculate_interval_above_max() {
            assert_eq!(Database::calculate_interval(10), 30);
        }
    }

    mod next_topic_tests {
        use super::*;

        #[test]
        fn get_next_topic_empty_db() {
            let db = setup_db();
            let next = db.get_next_topic(None).unwrap();
            assert!(next.is_none());
        }

        #[test]
        fn get_next_topic_returns_something() {
            let db = setup_db();
            db.add_topic("Topic A", None, &[]).unwrap();
            db.add_topic("Topic B", None, &[]).unwrap();

            let next = db.get_next_topic(None).unwrap();
            assert!(next.is_some());
        }

        #[test]
        fn get_next_topic_with_tag_filter() {
            let db = setup_db();
            db.add_topic("Rust Topic", None, &["rust".to_string()])
                .unwrap();
            db.add_topic("Go Topic", None, &["go".to_string()]).unwrap();

            // Filter by rust should return rust topic
            let next = db.get_next_topic(Some("rust")).unwrap().unwrap();
            assert_eq!(next.topic.name, "Rust Topic");

            // Filter by go should return go topic
            let next = db.get_next_topic(Some("go")).unwrap().unwrap();
            assert_eq!(next.topic.name, "Go Topic");
        }

        #[test]
        fn get_next_topic_filter_no_match() {
            let db = setup_db();
            db.add_topic("Topic", None, &["rust".to_string()]).unwrap();

            let next = db.get_next_topic(Some("python")).unwrap();
            assert!(next.is_none());
        }

        #[test]
        fn get_next_topic_includes_progress() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();
            db.record_review(id, ReviewOutcome::Success, None).unwrap();

            let next = db.get_next_topic(None).unwrap().unwrap();
            assert_eq!(next.progress.mastery_level, 1);
            assert_eq!(next.progress.times_reviewed, 1);
        }

        #[test]
        fn get_next_topic_includes_tags() {
            let db = setup_db();
            db.add_topic("Test", None, &["tag1".to_string(), "tag2".to_string()])
                .unwrap();

            let next = db.get_next_topic(None).unwrap().unwrap();
            assert_eq!(next.topic.tags.len(), 2);
        }
    }

    mod stats_tests {
        use super::*;

        #[test]
        fn stats_empty_db() {
            let db = setup_db();
            let stats = db.get_stats().unwrap();

            assert_eq!(stats.total_topics, 0);
            assert_eq!(stats.total_reviews, 0);
            assert_eq!(stats.mastered, 0);
            assert_eq!(stats.due_now, 0);
            assert_eq!(stats.avg_mastery, 0.0);
        }

        #[test]
        fn stats_counts_topics() {
            let db = setup_db();
            db.add_topic("A", None, &[]).unwrap();
            db.add_topic("B", None, &[]).unwrap();

            let stats = db.get_stats().unwrap();
            assert_eq!(stats.total_topics, 2);
        }

        #[test]
        fn stats_counts_reviews() {
            let db = setup_db();
            let id = db.add_topic("Test", None, &[]).unwrap();
            db.record_review(id, ReviewOutcome::Success, None).unwrap();
            db.record_review(id, ReviewOutcome::Partial, None).unwrap();
            db.record_review(id, ReviewOutcome::Fail, None).unwrap();

            let stats = db.get_stats().unwrap();
            assert_eq!(stats.total_reviews, 3);
        }

        #[test]
        fn stats_counts_mastered() {
            let db = setup_db();
            let id1 = db.add_topic("Master", None, &[]).unwrap();
            let id2 = db.add_topic("Novice", None, &[]).unwrap();

            // Get id1 to level 4 (proficient)
            for _ in 0..4 {
                db.record_review(id1, ReviewOutcome::Success, None).unwrap();
            }

            // id2 stays at level 0
            let _ = id2;

            let stats = db.get_stats().unwrap();
            assert_eq!(stats.mastered, 1);
        }

        #[test]
        fn stats_calculates_avg_mastery() {
            let db = setup_db();
            let id1 = db.add_topic("Topic 1", None, &[]).unwrap();
            let id2 = db.add_topic("Topic 2", None, &[]).unwrap();

            // id1 to level 2
            db.record_review(id1, ReviewOutcome::Success, None).unwrap();
            db.record_review(id1, ReviewOutcome::Success, None).unwrap();

            // id2 stays at level 0
            let _ = id2;

            let stats = db.get_stats().unwrap();
            assert_eq!(stats.avg_mastery, 1.0); // (2 + 0) / 2 = 1.0
        }

        #[test]
        fn stats_due_now_counts_new_topics() {
            let db = setup_db();
            // New topics are immediately due
            db.add_topic("A", None, &[]).unwrap();
            db.add_topic("B", None, &[]).unwrap();

            let stats = db.get_stats().unwrap();
            assert_eq!(stats.due_now, 2);
        }
    }

    mod session_tests {
        use super::*;

        #[test]
        fn start_session_creates_record() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            let session_id = db.start_session(topic_id, SessionType::Feynman).unwrap();
            assert!(session_id > 0);

            let session = db.get_session(session_id).unwrap().unwrap();
            assert_eq!(session.topic_id, topic_id);
            assert_eq!(session.session_type, SessionType::Feynman);
            assert!(session.ended_at.is_none());
            assert!(session.outcome.is_none());
        }

        #[test]
        fn start_session_records_skill_level() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            // Set skill level first
            db.update_skill_level(topic_id, SkillLevel::Intermediate, AssessmentMethod::SelfAssessed, None)
                .unwrap();

            let session_id = db.start_session(topic_id, SessionType::Socratic).unwrap();
            let session = db.get_session(session_id).unwrap().unwrap();
            assert_eq!(session.skill_level_at_start, Some(3)); // Intermediate = 3
        }

        #[test]
        fn end_session_updates_record() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();
            let session_id = db.start_session(topic_id, SessionType::Feynman).unwrap();

            db.end_session(
                session_id,
                SessionOutcome::Success,
                Some("Great session"),
                Some("User understood well"),
            )
            .unwrap();

            let session = db.get_session(session_id).unwrap().unwrap();
            assert!(session.ended_at.is_some());
            assert_eq!(session.outcome, Some(SessionOutcome::Success));
            assert_eq!(session.summary, Some("Great session".to_string()));
            assert_eq!(session.notes, Some("User understood well".to_string()));
        }

        #[test]
        fn list_sessions_returns_all_for_topic() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            db.start_session(topic_id, SessionType::Feynman).unwrap();
            db.start_session(topic_id, SessionType::Socratic).unwrap();

            let sessions = db.list_sessions(Some(topic_id)).unwrap();
            assert_eq!(sessions.len(), 2);
        }

        #[test]
        fn list_sessions_returns_all() {
            let db = setup_db();
            let t1 = db.add_topic("Test1", None, &[]).unwrap();
            let t2 = db.add_topic("Test2", None, &[]).unwrap();

            db.start_session(t1, SessionType::Feynman).unwrap();
            db.start_session(t2, SessionType::Socratic).unwrap();

            let sessions = db.list_sessions(None).unwrap();
            assert_eq!(sessions.len(), 2);
        }

        #[test]
        fn get_session_not_found() {
            let db = setup_db();
            let session = db.get_session(999).unwrap();
            assert!(session.is_none());
        }
    }

    mod session_gap_tests {
        use super::*;

        #[test]
        fn add_session_gap_creates_record() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();
            let session_id = db.start_session(topic_id, SessionType::Feynman).unwrap();

            let gap_id = db
                .add_session_gap(session_id, "Confused about ownership")
                .unwrap();
            assert!(gap_id > 0);

            let gaps = db.get_session_gaps(session_id).unwrap();
            assert_eq!(gaps.len(), 1);
            assert_eq!(gaps[0].gap_description, "Confused about ownership");
            assert!(!gaps[0].addressed);
        }

        #[test]
        fn mark_gap_addressed_updates_record() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();
            let session_id = db.start_session(topic_id, SessionType::Feynman).unwrap();
            let gap_id = db.add_session_gap(session_id, "Gap 1").unwrap();

            db.mark_gap_addressed(gap_id).unwrap();

            let gaps = db.get_session_gaps(session_id).unwrap();
            assert!(gaps[0].addressed);
        }

        #[test]
        fn get_unaddressed_gaps_filters_correctly() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();
            let session_id = db.start_session(topic_id, SessionType::Feynman).unwrap();

            let gap1 = db.add_session_gap(session_id, "Gap 1").unwrap();
            db.add_session_gap(session_id, "Gap 2").unwrap();

            db.mark_gap_addressed(gap1).unwrap();

            let unaddressed = db.get_unaddressed_gaps(topic_id).unwrap();
            assert_eq!(unaddressed.len(), 1);
            assert_eq!(unaddressed[0].gap_description, "Gap 2");
        }
    }

    mod skill_assessment_tests {
        use super::*;

        #[test]
        fn update_skill_level_changes_progress() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            db.update_skill_level(topic_id, SkillLevel::Intermediate, AssessmentMethod::SelfAssessed, None)
                .unwrap();

            let progress = db.get_progress(topic_id).unwrap().unwrap();
            assert_eq!(progress.skill_level, SkillLevel::Intermediate);
            assert_eq!(progress.assessment_method, AssessmentMethod::SelfAssessed);
            assert!(progress.last_assessed.is_some());
        }

        #[test]
        fn update_skill_level_creates_history() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            db.update_skill_level(topic_id, SkillLevel::Beginner, AssessmentMethod::Calibration, Some("Initial assessment"))
                .unwrap();

            db.update_skill_level(topic_id, SkillLevel::Intermediate, AssessmentMethod::Calibration, Some("Improved"))
                .unwrap();

            let assessments = db.get_skill_assessments(topic_id).unwrap();
            assert_eq!(assessments.len(), 2);

            // Most recent first
            assert_eq!(assessments[0].new_level, 3); // Intermediate
            assert_eq!(assessments[0].previous_level, Some(2)); // Beginner
            assert_eq!(assessments[1].new_level, 2); // Beginner
            assert_eq!(assessments[1].previous_level, Some(0)); // Unknown
        }

        #[test]
        fn get_skill_assessments_empty() {
            let db = setup_db();
            let topic_id = db.add_topic("Test", None, &[]).unwrap();

            let assessments = db.get_skill_assessments(topic_id).unwrap();
            assert!(assessments.is_empty());
        }
    }

    mod plan_tests {
        use super::*;

        #[test]
        fn create_plan_returns_id() {
            let db = setup_db();
            let plan_id = db.create_plan("Test Plan", "Build a thing").unwrap();
            assert!(plan_id > 0);
        }

        #[test]
        fn get_plan_returns_created_plan() {
            let db = setup_db();
            let plan_id = db.create_plan("My Plan", "Initial description").unwrap();

            let plan = db.get_plan(plan_id).unwrap().unwrap();
            assert_eq!(plan.title, "My Plan");
            assert_eq!(plan.initial_description, "Initial description");
            assert_eq!(plan.status, PlanStatus::Interviewing);
            assert!(plan.engineer_level.is_none());
            assert!(plan.spec_file_path.is_none());
        }

        #[test]
        fn get_plan_not_found() {
            let db = setup_db();
            let plan = db.get_plan(999).unwrap();
            assert!(plan.is_none());
        }

        #[test]
        fn list_plans_returns_all() {
            let db = setup_db();
            db.create_plan("Plan 1", "Desc 1").unwrap();
            db.create_plan("Plan 2", "Desc 2").unwrap();

            let plans = db.list_plans(None).unwrap();
            assert_eq!(plans.len(), 2);
        }

        #[test]
        fn list_plans_filters_by_status() {
            let db = setup_db();
            let p1 = db.create_plan("Plan 1", "Desc 1").unwrap();
            db.create_plan("Plan 2", "Desc 2").unwrap();

            db.update_plan_status(p1, PlanStatus::Complete).unwrap();

            let interviewing = db.list_plans(Some(PlanStatus::Interviewing)).unwrap();
            assert_eq!(interviewing.len(), 1);

            let complete = db.list_plans(Some(PlanStatus::Complete)).unwrap();
            assert_eq!(complete.len(), 1);
        }

        #[test]
        fn update_plan_status_changes_status() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.update_plan_status(plan_id, PlanStatus::SpecReady).unwrap();

            let plan = db.get_plan(plan_id).unwrap().unwrap();
            assert_eq!(plan.status, PlanStatus::SpecReady);
        }

        #[test]
        fn update_plan_engineer_level_sets_level() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.update_plan_engineer_level(plan_id, "staff").unwrap();

            let plan = db.get_plan(plan_id).unwrap().unwrap();
            assert_eq!(plan.engineer_level, Some("staff".to_string()));
        }

        #[test]
        fn update_plan_spec_path_sets_path_and_status() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.update_plan_spec_path(plan_id, "/path/to/spec.md").unwrap();

            let plan = db.get_plan(plan_id).unwrap().unwrap();
            assert_eq!(plan.spec_file_path, Some("/path/to/spec.md".to_string()));
            assert_eq!(plan.status, PlanStatus::SpecReady);
        }

        #[test]
        fn delete_plan_removes_plan() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            let deleted = db.delete_plan(plan_id).unwrap();
            assert!(deleted);

            let plan = db.get_plan(plan_id).unwrap();
            assert!(plan.is_none());
        }

        #[test]
        fn delete_plan_not_found() {
            let db = setup_db();
            let deleted = db.delete_plan(999).unwrap();
            assert!(!deleted);
        }
    }

    mod interview_entry_tests {
        use super::*;

        #[test]
        fn add_interview_entry_creates_record() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            let entry_id = db.add_interview_entry(
                plan_id,
                InterviewEntryType::Question,
                "What is the scope?",
                InterviewCategory::Scope,
            ).unwrap();

            assert!(entry_id > 0);
        }

        #[test]
        fn get_interview_entries_returns_all_for_plan() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.add_interview_entry(plan_id, InterviewEntryType::Question, "Q1", InterviewCategory::Scope).unwrap();
            db.add_interview_entry(plan_id, InterviewEntryType::Answer, "A1", InterviewCategory::Scope).unwrap();
            db.add_interview_entry(plan_id, InterviewEntryType::Question, "Q2", InterviewCategory::Security).unwrap();

            let entries = db.get_interview_entries(plan_id).unwrap();
            assert_eq!(entries.len(), 3);
        }

        #[test]
        fn get_interview_entries_ordered_by_time() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.add_interview_entry(plan_id, InterviewEntryType::Question, "First", InterviewCategory::Scope).unwrap();
            db.add_interview_entry(plan_id, InterviewEntryType::Answer, "Second", InterviewCategory::Scope).unwrap();

            let entries = db.get_interview_entries(plan_id).unwrap();
            assert_eq!(entries[0].content, "First");
            assert_eq!(entries[1].content, "Second");
        }

        #[test]
        fn get_interview_entries_by_category_filters() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.add_interview_entry(plan_id, InterviewEntryType::Question, "Security Q", InterviewCategory::Security).unwrap();
            db.add_interview_entry(plan_id, InterviewEntryType::Question, "Scope Q", InterviewCategory::Scope).unwrap();

            let security = db.get_interview_entries_by_category(plan_id, InterviewCategory::Security).unwrap();
            assert_eq!(security.len(), 1);
            assert_eq!(security[0].content, "Security Q");
        }

        #[test]
        fn delete_plan_cascades_to_entries() {
            let db = setup_db();
            let plan_id = db.create_plan("Plan", "Desc").unwrap();

            db.add_interview_entry(plan_id, InterviewEntryType::Question, "Q1", InterviewCategory::Scope).unwrap();

            db.delete_plan(plan_id).unwrap();

            // Entries should be gone
            let entries = db.get_interview_entries(plan_id).unwrap();
            assert!(entries.is_empty());
        }
    }
}
