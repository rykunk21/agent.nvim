use std::io::{Result, Error, ErrorKind};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::agent::chat_manager::{TasksDocument, Task};
use crate::spec::design::DesignManager;

/// Manages task list creation and tracking
pub struct TasksManager {
    spec_root: PathBuf,
}

impl TasksManager {
    pub fn new(spec_root: PathBuf) -> Self {
        TasksManager { spec_root }
    }

    /// Generate task list template
    pub fn generate_tasks_template(&self, _feature_name: &str) -> Result<String> {
        let template = format!(
            r#"# Implementation Plan

- [ ] 1. Set up project structure
  - Create basic project layout
  - Set up dependencies
  - _Requirements: 1.1_

- [ ]* 1.1 Write property test for setup
  - **Property 1: Setup completeness**
  - **Validates: Requirements 1.1**

- [ ] 2. Implement core functionality
  - Create main components
  - Implement business logic
  - _Requirements: 2.1, 2.2_

- [ ]* 2.1 Write property test for core functionality
  - **Property 2: Core behavior**
  - **Validates: Requirements 2.1**

- [ ] 3. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
"#
        );

        Ok(template)
    }

    /// Generate task list from design document
    pub fn generate_tasks_from_design(&self, _feature_name: &str, design_content: &str) -> Result<String> {
        let design_manager = DesignManager::new(self.spec_root.clone());
        let properties = design_manager.extract_correctness_properties(design_content);
        
        let mut tasks_content = format!(
            r#"# Implementation Plan

- [ ] 1. Set up project structure and dependencies
  - Create directory structure and build configuration
  - Set up testing framework and dependencies
  - Configure development environment
  - _Requirements: 1.1_

"#
        );

        // Generate implementation tasks based on design sections
        let sections = self.extract_design_sections(design_content);
        let mut task_number = 2;

        for section in sections {
            tasks_content.push_str(&format!(
                "- [ ] {}. Implement {}\n",
                task_number,
                section.name.to_lowercase()
            ));
            
            for detail in section.details {
                tasks_content.push_str(&format!("  - {}\n", detail));
            }
            
            tasks_content.push_str(&format!("  - _Requirements: {}_\n\n", section.requirements_refs.join(", ")));
            
            // Add property-based tests for this section
            let section_properties: Vec<_> = properties.iter()
                .filter(|p| section.requirements_refs.iter().any(|req| p.requirements_refs.contains(req)))
                .collect();
            
            for (i, property) in section_properties.iter().enumerate() {
                tasks_content.push_str(&format!(
                    "- [ ]* {}.{} Write property test for {}\n",
                    task_number,
                    i + 1,
                    property.name.to_lowercase()
                ));
                tasks_content.push_str(&format!("  - **Property {}: {}**\n", i + 1, property.name));
                tasks_content.push_str(&format!("  - **Validates: Requirements {}**\n\n", property.requirements_refs.join(", ")));
            }
            
            task_number += 1;
        }

        // Add checkpoint
        tasks_content.push_str(&format!(
            "- [ ] {}. Checkpoint - Ensure all tests pass\n",
            task_number
        ));
        tasks_content.push_str("  - Ensure all tests pass, ask the user if questions arise.\n\n");

        // Add integration and finalization tasks
        task_number += 1;
        tasks_content.push_str(&format!(
            r#"- [ ] {}. Integration and finalization
  - Integrate all components
  - Perform end-to-end testing
  - Add documentation and examples
  - _Requirements: All_

- [ ] {}. Final Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
"#,
            task_number, task_number + 1
        ));

        Ok(tasks_content)
    }

    /// Save tasks document to filesystem
    pub fn save_tasks(&self, feature_name: &str, content: &str) -> Result<PathBuf> {
        let feature_dir = self.spec_root.join(feature_name);
        fs::create_dir_all(&feature_dir)?;
        
        let tasks_path = feature_dir.join("tasks.md");
        fs::write(&tasks_path, content)?;
        
        Ok(tasks_path)
    }

    /// Load tasks document from filesystem
    pub fn load_tasks(&self, feature_name: &str) -> Result<TasksDocument> {
        let tasks_path = self.spec_root.join(feature_name).join("tasks.md");
        
        if !tasks_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Tasks document not found for feature: {}", feature_name)
            ));
        }
        
        let content = fs::read_to_string(&tasks_path)?;
        let metadata = fs::metadata(&tasks_path)?;
        
        // Parse tasks from content
        let tasks = self.parse_tasks_from_content(&content);
        
        Ok(TasksDocument {
            content,
            approved: false,
            tasks,
            created_at: metadata.created()
                .ok()
                .and_then(|t| DateTime::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0
                ))
                .unwrap_or_else(Utc::now),
            updated_at: metadata.modified()
                .ok()
                .and_then(|t| DateTime::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64, 0
                ))
                .unwrap_or_else(Utc::now),
        })
    }

    /// Update existing tasks document
    pub fn update_tasks(&self, feature_name: &str, content: &str) -> Result<()> {
        let tasks_path = self.spec_root.join(feature_name).join("tasks.md");
        
        if !tasks_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Tasks document not found for feature: {}", feature_name)
            ));
        }
        
        fs::write(&tasks_path, content)?;
        Ok(())
    }

    /// Parse tasks from markdown content
    pub fn parse_tasks_from_content(&self, content: &str) -> Vec<Task> {
        let mut tasks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines {
            let trimmed = line.trim();
            
            // Match task lines: - [ ] or - [x] followed by task description
            if (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]")) && !trimmed.contains("*") {
                let completed = trimmed.starts_with("- [x]");
                
                // Extract task ID and description
                let task_text = if trimmed.starts_with("- [x]") {
                    trimmed.trim_start_matches("- [x]").trim()
                } else {
                    trimmed.trim_start_matches("- [ ]").trim()
                };
                
                // Parse task ID (number at the beginning)
                let parts: Vec<&str> = task_text.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let task_id = parts[0].trim().to_string();
                    let description = parts[1].trim().to_string();
                    
                    tasks.push(Task {
                        id: task_id,
                        description,
                        completed,
                        dependencies: Vec::new(), // TODO: Parse dependencies
                        requirements_refs: Vec::new(), // TODO: Parse requirements refs
                    });
                }
            }
        }
        
        tasks
    }

    /// Parse task completion status
    pub fn parse_task_status(&self, task_line: &str) -> TaskStatus {
        if task_line.contains("- [x]") {
            TaskStatus::Completed
        } else if task_line.contains("- [ ]") {
            TaskStatus::NotStarted
        } else {
            TaskStatus::Unknown
        }
    }

    /// Update task completion status in content
    pub fn update_task_status(&self, content: &str, task_id: &str, status: TaskStatus) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut updated_lines = Vec::new();
        
        for line in lines {
            let trimmed = line.trim();
            
            // Check if this line contains the target task
            if (trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]")) && trimmed.contains(&format!("{}.", task_id)) {
                let task_text = if trimmed.starts_with("- [x]") {
                    trimmed.trim_start_matches("- [x]").trim()
                } else {
                    trimmed.trim_start_matches("- [ ]").trim()
                };
                
                let checkbox = match status {
                    TaskStatus::Completed => "- [x]",
                    TaskStatus::NotStarted => "- [ ]",
                    TaskStatus::InProgress => "- [ ]", // Use same as not started
                    TaskStatus::Unknown => "- [ ]",
                };
                
                // Preserve original indentation
                let indent = line.len() - line.trim_start().len();
                let indentation = " ".repeat(indent);
                updated_lines.push(format!("{}{} {}", indentation, checkbox, task_text));
            } else {
                updated_lines.push(line.to_string());
            }
        }
        
        Ok(updated_lines.join("\n"))
    }

    /// Extract design sections for task generation
    fn extract_design_sections(&self, design_content: &str) -> Vec<DesignSection> {
        let mut sections = Vec::new();
        let lines: Vec<&str> = design_content.lines().collect();
        
        let mut current_section: Option<DesignSection> = None;
        
        for line in lines {
            if line.starts_with("## ") && !line.contains("Correctness Properties") && !line.contains("Testing Strategy") {
                // Save previous section
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }
                
                // Start new section
                let section_name = line.trim_start_matches("## ").trim().to_string();
                current_section = Some(DesignSection {
                    name: section_name,
                    details: Vec::new(),
                    requirements_refs: vec!["TBD".to_string()], // TODO: Extract from content
                });
            }
            
            // Add implementation details from section content
            if let Some(ref mut section) = current_section {
                if line.trim().starts_with("- ") {
                    section.details.push(line.trim().trim_start_matches("- ").to_string());
                }
            }
        }
        
        // Don't forget the last section
        if let Some(section) = current_section {
            sections.push(section);
        }
        
        // Filter out sections that don't need implementation tasks
        sections.into_iter()
            .filter(|s| !["Overview", "Error Handling", "Testing Strategy"].contains(&s.name.as_str()))
            .collect()
    }

    /// Get task completion statistics
    pub fn get_completion_stats(&self, content: &str) -> TaskStats {
        let tasks = self.parse_tasks_from_content(content);
        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.completed).count();
        let remaining = total - completed;
        let completion_percentage = if total > 0 {
            (completed as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        
        TaskStats {
            total,
            completed,
            remaining,
            completion_percentage,
        }
    }

    /// Check if tasks document exists
    pub fn exists(&self, feature_name: &str) -> bool {
        self.spec_root.join(feature_name).join("tasks.md").exists()
    }
}

/// Task completion status
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Unknown,
}

/// Design section for task generation
#[derive(Debug, Clone)]
struct DesignSection {
    name: String,
    details: Vec<String>,
    requirements_refs: Vec<String>,
}

/// Task completion statistics
#[derive(Debug, Clone)]
pub struct TaskStats {
    pub total: usize,
    pub completed: usize,
    pub remaining: usize,
    pub completion_percentage: f32,
}