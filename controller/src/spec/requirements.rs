use std::io::{Result, Error, ErrorKind};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::agent::chat_manager::RequirementsDocument;

/// Manages EARS-compliant requirements documents
pub struct RequirementsManager {
    spec_root: PathBuf,
}

impl RequirementsManager {
    pub fn new(spec_root: PathBuf) -> Self {
        RequirementsManager { spec_root }
    }

    /// Create a new requirements document from template
    pub fn create_requirements(&self, feature_name: &str) -> Result<String> {
        let template = format!(
            r#"# Requirements Document

## Introduction

This document specifies the requirements for {}.

## Glossary

- **System**: The main application or component being developed

## Requirements

### Requirement 1

**User Story:** As a user, I want [feature], so that [benefit]

#### Acceptance Criteria

1. WHEN [trigger] THEN the System SHALL [response]
2. WHILE [condition] THE System SHALL [response]
3. IF [unwanted event] THEN the System SHALL [response]
"#,
            feature_name
        );

        Ok(template)
    }

    /// Save requirements document to filesystem
    pub fn save_requirements(&self, feature_name: &str, content: &str) -> Result<PathBuf> {
        let feature_dir = self.spec_root.join(feature_name);
        fs::create_dir_all(&feature_dir)?;
        
        let requirements_path = feature_dir.join("requirements.md");
        fs::write(&requirements_path, content)?;
        
        Ok(requirements_path)
    }

    /// Load requirements document from filesystem
    pub fn load_requirements(&self, feature_name: &str) -> Result<RequirementsDocument> {
        let requirements_path = self.spec_root.join(feature_name).join("requirements.md");
        
        if !requirements_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Requirements document not found for feature: {}", feature_name)
            ));
        }
        
        let content = fs::read_to_string(&requirements_path)?;
        let metadata = fs::metadata(&requirements_path)?;
        
        Ok(RequirementsDocument {
            content,
            approved: false, // Default to not approved when loading
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

    /// Update existing requirements document
    pub fn update_requirements(&self, feature_name: &str, content: &str) -> Result<()> {
        let requirements_path = self.spec_root.join(feature_name).join("requirements.md");
        
        if !requirements_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Requirements document not found for feature: {}", feature_name)
            ));
        }
        
        fs::write(&requirements_path, content)?;
        Ok(())
    }

    /// Validate EARS compliance for a single requirement
    pub fn validate_ears_compliance(&self, requirement: &str) -> bool {
        let ears_patterns = [
            "THE", "SHALL", // Basic pattern
            "WHEN", "WHILE", "IF", "WHERE", // Conditional patterns
        ];

        ears_patterns.iter().any(|pattern| requirement.contains(pattern))
    }

    /// Validate EARS pattern structure
    pub fn validate_ears_pattern(&self, requirement: &str) -> EarsValidation {
        let requirement_upper = requirement.to_uppercase();
        
        // Check for SHALL keyword (required)
        if !requirement_upper.contains("SHALL") {
            return EarsValidation::Invalid("Missing SHALL keyword".to_string());
        }
        
        // Check for THE keyword (required)
        if !requirement_upper.contains("THE") {
            return EarsValidation::Invalid("Missing THE keyword".to_string());
        }
        
        // Identify pattern type
        let has_when = requirement_upper.contains("WHEN");
        let has_while = requirement_upper.contains("WHILE");
        let has_if = requirement_upper.contains("IF");
        let has_where = requirement_upper.contains("WHERE");
        
        let pattern = if has_where && has_while && (has_when || has_if) {
            EarsPattern::Complex
        } else if has_when {
            EarsPattern::EventDriven
        } else if has_while {
            EarsPattern::StateDriven
        } else if has_if {
            EarsPattern::UnwantedEvent
        } else if has_where {
            EarsPattern::OptionalFeature
        } else {
            EarsPattern::Ubiquitous
        };
        
        EarsValidation::Valid(pattern)
    }

    /// Extract acceptance criteria from requirements document
    pub fn extract_acceptance_criteria(&self, content: &str) -> Vec<AcceptanceCriterion> {
        let mut criteria = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut current_requirement = None;
        let mut criterion_number = 0;
        
        for line in lines {
            // Detect requirement headers
            if line.starts_with("### Requirement") {
                current_requirement = Some(line.trim_start_matches("### Requirement").trim().to_string());
                criterion_number = 0;
            }
            
            // Detect acceptance criteria
            if let Some(req_id) = &current_requirement {
                if line.trim().starts_with(char::is_numeric) && line.contains("SHALL") {
                    criterion_number += 1;
                    let criterion_id = format!("{}.{}", req_id, criterion_number);
                    
                    criteria.push(AcceptanceCriterion {
                        id: criterion_id,
                        requirement_id: req_id.clone(),
                        text: line.trim().to_string(),
                        ears_pattern: self.validate_ears_pattern(line),
                    });
                }
            }
        }
        
        criteria
    }

    /// Validate entire requirements document
    pub fn validate_document(&self, content: &str) -> DocumentValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for required sections
        if !content.contains("## Introduction") {
            errors.push("Missing Introduction section".to_string());
        }
        
        if !content.contains("## Glossary") {
            warnings.push("Missing Glossary section (recommended)".to_string());
        }
        
        if !content.contains("## Requirements") {
            errors.push("Missing Requirements section".to_string());
        }
        
        // Validate acceptance criteria
        let criteria = self.extract_acceptance_criteria(content);
        for criterion in criteria {
            if let EarsValidation::Invalid(reason) = criterion.ears_pattern {
                errors.push(format!("Criterion {}: {}", criterion.id, reason));
            }
        }
        
        DocumentValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Check if requirements document exists
    pub fn exists(&self, feature_name: &str) -> bool {
        self.spec_root.join(feature_name).join("requirements.md").exists()
    }

    /// List all feature names with requirements
    pub fn list_features(&self) -> Result<Vec<String>> {
        let mut features = Vec::new();
        
        if !self.spec_root.exists() {
            return Ok(features);
        }
        
        for entry in fs::read_dir(&self.spec_root)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if path.join("requirements.md").exists() {
                    if let Some(name) = path.file_name() {
                        features.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        Ok(features)
    }
}

/// EARS pattern types
#[derive(Debug, Clone, PartialEq)]
pub enum EarsPattern {
    Ubiquitous,      // THE <system> SHALL <response>
    EventDriven,     // WHEN <trigger>, THE <system> SHALL <response>
    StateDriven,     // WHILE <condition>, THE <system> SHALL <response>
    UnwantedEvent,   // IF <condition>, THEN THE <system> SHALL <response>
    OptionalFeature, // WHERE <option>, THE <system> SHALL <response>
    Complex,         // Combination of patterns
}

/// EARS validation result
#[derive(Debug, Clone, PartialEq)]
pub enum EarsValidation {
    Valid(EarsPattern),
    Invalid(String),
}

/// Acceptance criterion structure
#[derive(Debug, Clone)]
pub struct AcceptanceCriterion {
    pub id: String,
    pub requirement_id: String,
    pub text: String,
    pub ears_pattern: EarsValidation,
}

/// Document validation result
#[derive(Debug, Clone)]
pub struct DocumentValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}