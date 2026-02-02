mod db;
mod models;
mod tui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use db::Database;
use models::{JsonOutput, ReviewOutcome};

const DEFAULT_DB_NAME: &str = "feynman.db";

#[derive(Parser)]
#[command(name = "feynman")]
#[command(about = "A stochastic teacher CLI using Feynman techniques for deep learning")]
#[command(version)]
struct Cli {
    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database
    Init,

    /// Manage topics
    #[command(subcommand)]
    Topic(TopicCommands),

    /// List all tags
    Tags,

    /// Show learning statistics
    Stats,

    /// Get next topic to review (stochastic selection)
    Next {
        /// Filter by tag
        #[arg(long, short)]
        tag: Option<String>,
    },

    /// Record a review outcome for a topic
    Review {
        /// Topic ID
        id: i64,

        /// Review outcome: success/partial/fail
        #[arg(long, short)]
        outcome: String,

        /// Optional notes about the review
        #[arg(long, short)]
        notes: Option<String>,
    },

    /// Launch interactive terminal UI
    Tui,
}

#[derive(Subcommand)]
enum TopicCommands {
    /// List all topics
    List {
        /// Filter by tag
        #[arg(long, short)]
        tag: Option<String>,
    },

    /// Add a new topic
    Add {
        /// Topic name
        name: String,

        /// Topic description
        #[arg(long, short)]
        description: Option<String>,

        /// Comma-separated tags
        #[arg(long, short)]
        tags: Option<String>,
    },

    /// Show topic details
    Show {
        /// Topic ID
        id: i64,
    },

    /// Delete a topic
    Delete {
        /// Topic ID
        id: i64,
    },

    /// Update topic tags
    Tag {
        /// Topic ID
        id: i64,

        /// Comma-separated tags (replaces existing)
        #[arg(long, short)]
        tags: String,
    },
}

fn get_db_path() -> PathBuf {
    if let Ok(path) = std::env::var("FEYNMAN_DB") {
        return PathBuf::from(path);
    }

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("feynman");

    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join(DEFAULT_DB_NAME)
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_db_path();
    let db = Database::open(&db_path)?;

    match cli.command {
        Commands::Init => {
            db.init()?;
            if cli.json {
                println!("{}", serde_json::to_string(&JsonOutput::<()>::ok(()))?);
            } else {
                println!("Database initialized at: {}", db_path.display());
            }
        }

        Commands::Topic(topic_cmd) => match topic_cmd {
            TopicCommands::List { tag } => {
                let topics = db.list_topics(tag.as_deref())?;
                if cli.json {
                    println!("{}", serde_json::to_string(&JsonOutput::ok(&topics))?);
                } else if topics.is_empty() {
                    println!("No topics found.");
                } else {
                    println!("{:<5} {:<40} TAGS", "ID", "NAME");
                    println!("{}", "-".repeat(70));
                    for topic in topics {
                        let tags = if topic.tags.is_empty() {
                            String::from("-")
                        } else {
                            topic.tags.join(", ")
                        };
                        println!("{:<5} {:<40} {}", topic.id, truncate(&topic.name, 38), tags);
                    }
                }
            }

            TopicCommands::Add {
                name,
                description,
                tags,
            } => {
                let tag_list: Vec<String> = tags
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                let id = db.add_topic(&name, description.as_deref(), &tag_list)?;

                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string(&JsonOutput::ok(serde_json::json!({
                            "id": id,
                            "name": name
                        })))?
                    );
                } else {
                    println!("Added topic '{}' with ID: {}", name, id);
                }
            }

            TopicCommands::Show { id } => {
                if let Some(topic) = db.get_topic(id)? {
                    let progress = db.get_progress(id)?;

                    if cli.json {
                        println!(
                            "{}",
                            serde_json::to_string(&JsonOutput::ok(serde_json::json!({
                                "topic": topic,
                                "progress": progress
                            })))?
                        );
                    } else {
                        println!("Topic: {}", topic.name);
                        println!("ID: {}", topic.id);
                        if let Some(desc) = &topic.description {
                            println!("Description: {}", desc);
                        }
                        println!(
                            "Tags: {}",
                            if topic.tags.is_empty() {
                                "-".to_string()
                            } else {
                                topic.tags.join(", ")
                            }
                        );
                        println!("Created: {}", topic.created_at);

                        if let Some(p) = progress {
                            println!();
                            println!("--- Progress ---");
                            println!("Mastery: {} (level {})", p.mastery_label(), p.mastery_level);
                            println!(
                                "Reviews: {} ({:.0}% success rate)",
                                p.times_reviewed,
                                p.success_rate()
                            );
                            if let Some(last) = &p.last_reviewed {
                                println!("Last reviewed: {}", last);
                            }
                            if let Some(next) = &p.next_review {
                                println!("Next review: {}", next);
                            }
                        }
                    }
                } else if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string(&JsonOutput::<()>::err("Topic not found"))?
                    );
                } else {
                    println!("Topic not found.");
                }
            }

            TopicCommands::Delete { id } => {
                if db.delete_topic(id)? {
                    if cli.json {
                        println!("{}", serde_json::to_string(&JsonOutput::<()>::ok(()))?);
                    } else {
                        println!("Topic {} deleted.", id);
                    }
                } else if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string(&JsonOutput::<()>::err("Topic not found"))?
                    );
                } else {
                    println!("Topic not found.");
                }
            }

            TopicCommands::Tag { id, tags } => {
                let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).collect();
                db.update_topic_tags(id, &tag_list)?;

                if cli.json {
                    println!("{}", serde_json::to_string(&JsonOutput::<()>::ok(()))?);
                } else {
                    println!("Updated tags for topic {}.", id);
                }
            }
        },

        Commands::Tags => {
            let tags = db.list_tags()?;
            if cli.json {
                println!("{}", serde_json::to_string(&JsonOutput::ok(&tags))?);
            } else if tags.is_empty() {
                println!("No tags found.");
            } else {
                println!("{:<5} {:<30} TOPICS", "ID", "TAG");
                println!("{}", "-".repeat(50));
                for tag in tags {
                    println!("{:<5} {:<30} {}", tag.id, tag.name, tag.topic_count);
                }
            }
        }

        Commands::Stats => {
            let stats = db.get_stats()?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string(&JsonOutput::ok(serde_json::json!({
                        "total_topics": stats.total_topics,
                        "total_reviews": stats.total_reviews,
                        "mastered": stats.mastered,
                        "due_now": stats.due_now,
                        "avg_mastery": stats.avg_mastery
                    })))?
                );
            } else {
                println!("=== Learning Statistics ===");
                println!("Total topics: {}", stats.total_topics);
                println!("Total reviews: {}", stats.total_reviews);
                println!("Mastered (level 4+): {}", stats.mastered);
                println!("Due for review: {}", stats.due_now);
                println!("Average mastery: {:.1}/5", stats.avg_mastery);
            }
        }

        Commands::Next { tag } => {
            if let Some(twp) = db.get_next_topic(tag.as_deref())? {
                if cli.json {
                    println!("{}", serde_json::to_string(&JsonOutput::ok(&twp))?);
                } else {
                    println!("=== Next Topic to Review ===");
                    println!();
                    println!("Topic: {} (ID: {})", twp.topic.name, twp.topic.id);
                    if let Some(desc) = &twp.topic.description {
                        println!("Description: {}", desc);
                    }
                    println!(
                        "Tags: {}",
                        if twp.topic.tags.is_empty() {
                            "-".to_string()
                        } else {
                            twp.topic.tags.join(", ")
                        }
                    );
                    println!();
                    println!(
                        "Current mastery: {} (level {})",
                        twp.progress.mastery_label(),
                        twp.progress.mastery_level
                    );
                    println!(
                        "Reviews: {} ({:.0}% success)",
                        twp.progress.times_reviewed,
                        twp.progress.success_rate()
                    );
                    println!();
                    println!("After review, record outcome with:");
                    println!(
                        "  feynman review {} --outcome <success|partial|fail>",
                        twp.topic.id
                    );
                }
            } else if cli.json {
                println!("{}", serde_json::to_string(&JsonOutput::<()>::ok(()))?);
            } else {
                println!("No topics to review. Add some topics first!");
            }
        }

        Commands::Review { id, outcome, notes } => {
            let review_outcome = ReviewOutcome::from_str(&outcome).ok_or_else(|| {
                format!(
                    "Invalid outcome '{}'. Use: success, partial, or fail",
                    outcome
                )
            })?;

            db.record_review(id, review_outcome, notes.as_deref())?;

            if cli.json {
                println!("{}", serde_json::to_string(&JsonOutput::<()>::ok(()))?);
            } else {
                println!("Review recorded for topic {}.", id);
                if let Some(progress) = db.get_progress(id)? {
                    println!(
                        "New mastery level: {} ({})",
                        progress.mastery_level,
                        progress.mastery_label()
                    );
                    if let Some(next) = &progress.next_review {
                        println!("Next review scheduled: {}", next);
                    }
                }
            }
        }

        Commands::Tui => {
            tui::run(db)?;
        }
    }

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    mod truncate_tests {
        use super::*;

        #[test]
        fn truncate_short_string() {
            assert_eq!(truncate("hello", 10), "hello");
        }

        #[test]
        fn truncate_exact_length() {
            assert_eq!(truncate("hello", 5), "hello");
        }

        #[test]
        fn truncate_long_string() {
            assert_eq!(truncate("hello world", 8), "hello...");
        }

        #[test]
        fn truncate_empty_string() {
            assert_eq!(truncate("", 10), "");
        }

        #[test]
        fn truncate_minimum_length() {
            // With max_len = 4, we get 1 char + "..."
            assert_eq!(truncate("hello", 4), "h...");
        }
    }

    mod cli_parsing_tests {
        use super::*;

        #[test]
        fn parse_init_command() {
            let cli = Cli::try_parse_from(["feynman", "init"]).unwrap();
            assert!(!cli.json);
            assert!(matches!(cli.command, Commands::Init));
        }

        #[test]
        fn parse_init_with_json() {
            let cli = Cli::try_parse_from(["feynman", "--json", "init"]).unwrap();
            assert!(cli.json);
            assert!(matches!(cli.command, Commands::Init));
        }

        #[test]
        fn parse_topic_list() {
            let cli = Cli::try_parse_from(["feynman", "topic", "list"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::List { tag }) => {
                    assert!(tag.is_none());
                }
                _ => panic!("Expected Topic List command"),
            }
        }

        #[test]
        fn parse_topic_list_with_tag() {
            let cli = Cli::try_parse_from(["feynman", "topic", "list", "--tag", "rust"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::List { tag }) => {
                    assert_eq!(tag, Some("rust".to_string()));
                }
                _ => panic!("Expected Topic List command"),
            }
        }

        #[test]
        fn parse_topic_list_with_tag_short() {
            let cli = Cli::try_parse_from(["feynman", "topic", "list", "-t", "rust"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::List { tag }) => {
                    assert_eq!(tag, Some("rust".to_string()));
                }
                _ => panic!("Expected Topic List command"),
            }
        }

        #[test]
        fn parse_topic_add_basic() {
            let cli = Cli::try_parse_from(["feynman", "topic", "add", "Rust Basics"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Add {
                    name,
                    description,
                    tags,
                }) => {
                    assert_eq!(name, "Rust Basics");
                    assert!(description.is_none());
                    assert!(tags.is_none());
                }
                _ => panic!("Expected Topic Add command"),
            }
        }

        #[test]
        fn parse_topic_add_with_description() {
            let cli = Cli::try_parse_from([
                "feynman",
                "topic",
                "add",
                "Rust Basics",
                "--description",
                "Learn fundamentals",
            ])
            .unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Add {
                    name,
                    description,
                    tags,
                }) => {
                    assert_eq!(name, "Rust Basics");
                    assert_eq!(description, Some("Learn fundamentals".to_string()));
                    assert!(tags.is_none());
                }
                _ => panic!("Expected Topic Add command"),
            }
        }

        #[test]
        fn parse_topic_add_with_tags() {
            let cli = Cli::try_parse_from([
                "feynman",
                "topic",
                "add",
                "Rust Basics",
                "--tags",
                "rust,programming",
            ])
            .unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Add {
                    name,
                    description,
                    tags,
                }) => {
                    assert_eq!(name, "Rust Basics");
                    assert!(description.is_none());
                    assert_eq!(tags, Some("rust,programming".to_string()));
                }
                _ => panic!("Expected Topic Add command"),
            }
        }

        #[test]
        fn parse_topic_add_full() {
            let cli = Cli::try_parse_from([
                "feynman",
                "topic",
                "add",
                "Rust Basics",
                "-d",
                "Description",
                "-t",
                "tag1,tag2",
            ])
            .unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Add {
                    name,
                    description,
                    tags,
                }) => {
                    assert_eq!(name, "Rust Basics");
                    assert_eq!(description, Some("Description".to_string()));
                    assert_eq!(tags, Some("tag1,tag2".to_string()));
                }
                _ => panic!("Expected Topic Add command"),
            }
        }

        #[test]
        fn parse_topic_show() {
            let cli = Cli::try_parse_from(["feynman", "topic", "show", "42"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Show { id }) => {
                    assert_eq!(id, 42);
                }
                _ => panic!("Expected Topic Show command"),
            }
        }

        #[test]
        fn parse_topic_delete() {
            let cli = Cli::try_parse_from(["feynman", "topic", "delete", "5"]).unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Delete { id }) => {
                    assert_eq!(id, 5);
                }
                _ => panic!("Expected Topic Delete command"),
            }
        }

        #[test]
        fn parse_topic_tag() {
            let cli = Cli::try_parse_from(["feynman", "topic", "tag", "3", "--tags", "new,tags"])
                .unwrap();
            match cli.command {
                Commands::Topic(TopicCommands::Tag { id, tags }) => {
                    assert_eq!(id, 3);
                    assert_eq!(tags, "new,tags");
                }
                _ => panic!("Expected Topic Tag command"),
            }
        }

        #[test]
        fn parse_tags_command() {
            let cli = Cli::try_parse_from(["feynman", "tags"]).unwrap();
            assert!(matches!(cli.command, Commands::Tags));
        }

        #[test]
        fn parse_stats_command() {
            let cli = Cli::try_parse_from(["feynman", "stats"]).unwrap();
            assert!(matches!(cli.command, Commands::Stats));
        }

        #[test]
        fn parse_next_command() {
            let cli = Cli::try_parse_from(["feynman", "next"]).unwrap();
            match cli.command {
                Commands::Next { tag } => {
                    assert!(tag.is_none());
                }
                _ => panic!("Expected Next command"),
            }
        }

        #[test]
        fn parse_next_with_tag() {
            let cli = Cli::try_parse_from(["feynman", "next", "--tag", "rust"]).unwrap();
            match cli.command {
                Commands::Next { tag } => {
                    assert_eq!(tag, Some("rust".to_string()));
                }
                _ => panic!("Expected Next command"),
            }
        }

        #[test]
        fn parse_review_command() {
            let cli =
                Cli::try_parse_from(["feynman", "review", "7", "--outcome", "success"]).unwrap();
            match cli.command {
                Commands::Review { id, outcome, notes } => {
                    assert_eq!(id, 7);
                    assert_eq!(outcome, "success");
                    assert!(notes.is_none());
                }
                _ => panic!("Expected Review command"),
            }
        }

        #[test]
        fn parse_review_with_notes() {
            let cli = Cli::try_parse_from([
                "feynman",
                "review",
                "7",
                "--outcome",
                "partial",
                "--notes",
                "Struggled with X",
            ])
            .unwrap();
            match cli.command {
                Commands::Review { id, outcome, notes } => {
                    assert_eq!(id, 7);
                    assert_eq!(outcome, "partial");
                    assert_eq!(notes, Some("Struggled with X".to_string()));
                }
                _ => panic!("Expected Review command"),
            }
        }

        #[test]
        fn parse_review_short_flags() {
            let cli = Cli::try_parse_from(["feynman", "review", "1", "-o", "fail", "-n", "notes"])
                .unwrap();
            match cli.command {
                Commands::Review { id, outcome, notes } => {
                    assert_eq!(id, 1);
                    assert_eq!(outcome, "fail");
                    assert_eq!(notes, Some("notes".to_string()));
                }
                _ => panic!("Expected Review command"),
            }
        }

        #[test]
        fn parse_json_flag_global() {
            // JSON flag works regardless of position
            let cli1 = Cli::try_parse_from(["feynman", "--json", "stats"]).unwrap();
            assert!(cli1.json);

            // Also works after subcommand in some cases
            let cli2 = Cli::try_parse_from(["feynman", "stats", "--json"]).unwrap();
            assert!(cli2.json);
        }

        #[test]
        fn parse_invalid_command_fails() {
            let result = Cli::try_parse_from(["feynman", "invalid"]);
            assert!(result.is_err());
        }

        #[test]
        fn parse_missing_required_arg_fails() {
            // topic add requires name
            let result = Cli::try_parse_from(["feynman", "topic", "add"]);
            assert!(result.is_err());

            // review requires id and outcome
            let result = Cli::try_parse_from(["feynman", "review"]);
            assert!(result.is_err());

            let result = Cli::try_parse_from(["feynman", "review", "1"]);
            assert!(result.is_err());
        }
    }

    mod db_path_tests {
        use super::*;
        use std::env;

        #[test]
        fn get_db_path_uses_env_var() {
            let test_path = "/tmp/test_feynman.db";
            env::set_var("FEYNMAN_DB", test_path);

            let path = get_db_path();
            assert_eq!(path.to_str().unwrap(), test_path);

            env::remove_var("FEYNMAN_DB");
        }

        #[test]
        fn get_db_path_default_includes_feynman_db() {
            env::remove_var("FEYNMAN_DB");

            let path = get_db_path();
            let path_str = path.to_str().unwrap();

            assert!(path_str.ends_with("feynman.db"));
            assert!(path_str.contains("feynman"));
        }
    }
}
