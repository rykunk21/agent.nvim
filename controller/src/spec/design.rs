use std::io::{Result, Error, ErrorKind};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::agent::chat_manager::{DesignDocument, CorrectnessProperty, PropertyType};
use crate::spec::requirements::{RequirementsManager, AcceptanceCriterion};

/// Manages design document generation and validation
pub struct DesignManager {
    spec_root: PathBuf,
}

impl DesignManager {
    pub fn new(spec_root: PathBuf) -> Self {
        DesignManager { spec_root }
    }

    /// Generate design document template from requirements
    pub fn generate_design_template(&self, feature_name: &str) -> Result<String> {
        let template = format!(
            r#"# Design Document: {}

## Overview

[Feature overview and purpose]

## Architecture

[High-level architecture description]

## Components and Interfaces

[Component descriptions and interfaces]

## Data Models

[Data structures and models]

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system.*

Property 1: [Property description]
*For any* [input domain], [expected behavior]
**Validates: Requirements X.Y**

## Error Handling

[Error handling strategies]

## Testing Strategy

[Testing approach including unit and property-based tests]
"#,
            feature_name
        );

        Ok(template)
    }

    /// Generate design document from requirements analysis
    pub fn generate_design_from_requirements(&self, feature_name: &str, requirements_content: &str) -> Result<String> {
        let requirements_manager = RequirementsManager::new(self.spec_root.clone());
        let criteria = requirements_manager.extract_acceptance_criteria(requirements_content);
        
        let mut design_content = format!(
            r#"# Design Document: {}

## Overview

This design document outlines the architecture and implementation approach for {}.

## Architecture

[Architecture will be defined based on requirements analysis]

## Components and Interfaces

[Components will be identified from requirements]

## Data Models

[Data models will be derived from functional requirements]

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system.*

"#,
            feature_name, feature_name
        );

        // Generate correctness properties from acceptance criteria
        let properties = self.generate_correctness_properties(&criteria);
        for (i, property) in properties.iter().enumerate() {
            design_content.push_str(&format!(
                "Property {}: {}\n*For any* {}\n**Validates: Requirements {}**\n\n",
                i + 1,
                property.name,
                property.description,
                property.requirements_refs.join(", ")
            ));
        }

        design_content.push_str(
            r#"## Error Handling

[Error handling strategies based on requirements]

## Testing Strategy

### Dual Testing Approach

The implementation will use both unit testing and property-based testing:

**Unit Testing**
- Specific examples that demonstrate correct behavior
- Edge cases and error conditions
- Integration points between components

**Property-Based Testing**
- Universal properties that should hold across all inputs
- Each correctness property will be implemented as a property-based test
- Minimum 100 iterations per property test
- Tests will be tagged with property references

**Testing Framework**
- Unit tests using standard testing framework
- Property-based tests using appropriate PBT library
- Each property test tagged with: **Feature: {}, Property N: [property_text]**
"#
        );

        Ok(design_content)
    }

    /// Save design document to filesystem
    pub fn save_design(&self, feature_name: &str, content: &str) -> Result<PathBuf> {
        let feature_dir = self.spec_root.join(feature_name);
        fs::create_dir_all(&feature_dir)?;
        
        let design_path = feature_dir.join("design.md");
        fs::write(&design_path, content)?;
        
        Ok(design_path)
    }

    /// Load design document from filesystem
    pub fn load_design(&self, feature_name: &str) -> Result<DesignDocument> {
        let design_path = self.spec_root.join(feature_name).join("design.md");
        
        if !design_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Design document not found for feature: {}", feature_name)
            ));
        }
        
        let content = fs::read_to_string(&design_path)?;
        let metadata = fs::metadata(&design_path)?;
        
        // Extract correctness properties from content
        let properties = self.extract_correctness_properties(&content);
        
        Ok(DesignDocument {
            content,
            approved: false,
            correctness_properties: properties,
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

    /// Update existing design document
    pub fn update_design(&self, feature_name: &str, content: &str) -> Result<()> {
        let design_path = self.spec_root.join(feature_name).join("design.md");
        
        if !design_path.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Design document not found for feature: {}", feature_name)
            ));
        }
        
        fs::write(&design_path, content)?;
        Ok(())
    }

    /// Generate correctness properties from acceptance criteria
    pub fn generate_correctness_properties(&self, criteria: &[AcceptanceCriterion]) -> Vec<CorrectnessProperty> {
        let mut properties = Vec::new();
        
        for criterion in criteria {
            // Analyze criterion text to determine property type
            let property_type = self.determine_property_type(&criterion.text);
            
            let property = CorrectnessProperty {
                id: format!("property_{}", criterion.id.replace(".", "_")),
                name: self.generate_property_name(&criterion.text),
                description: self.generate_property_description(&criterion.text),
                property_type,
                requirements_refs: vec![criterion.id.clone()],
            };
            
            properties.push(property);
        }
        
        properties
    }

    /// Determine property type from criterion text
    fn determine_property_type(&self, criterion_text: &str) -> PropertyType {
        let text_lower = criterion_text.to_lowercase();
        
        if text_lower.contains("round") && text_lower.contains("trip") {
            PropertyType::RoundTrip
        } else if text_lower.contains("invariant") || text_lower.contains("preserve") {
            PropertyType::Invariant
        } else if text_lower.contains("idempotent") || text_lower.contains("same result") {
            PropertyType::Idempotence
        } else if text_lower.contains("error") || text_lower.contains("fail") {
            PropertyType::ErrorCondition
        } else if text_lower.contains("order") && text_lower.contains("independent") {
            PropertyType::Confluence
        } else if text_lower.contains("model") || text_lower.contains("reference") {
            PropertyType::ModelBased
        } else {
            PropertyType::Metamorphic
        }
    }

    /// Generate property name from criterion
    fn generate_property_name(&self, criterion_text: &str) -> String {
        // Extract key concepts from the criterion
        let words: Vec<&str> = criterion_text.split_whitespace().collect();
        let mut key_words = Vec::new();
        
        for word in words {
            let clean_word = word.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
            if clean_word.len() > 3 && !["when", "then", "shall", "the", "system"].contains(&clean_word.as_str()) {
                key_words.push(clean_word);
                if key_words.len() >= 3 {
                    break;
                }
            }
        }
        
        if key_words.is_empty() {
            "Generated property".to_string()
        } else {
            key_words.join(" ")
        }
    }

    /// Generate property description from criterion
    fn generate_property_description(&self, criterion_text: &str) -> String {
        // Convert EARS format to property format
        let text = criterion_text.replace("WHEN", "when")
            .replace("THEN", "then")
            .replace("THE", "the")
            .replace("SHALL", "should");
        
        format!("Property derived from: {}", text)
    }

    /// Extract correctness properties from design document content
    pub fn extract_correctness_properties(&self, content: &str) -> Vec<CorrectnessProperty> {
        let mut properties = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let mut in_properties_section = false;
        let mut current_property: Option<CorrectnessProperty> = None;
        
        for line in lines {
            if line.starts_with("## Correctness Properties") {
                in_properties_section = true;
                continue;
            }
            
            if in_properties_section && line.starts_with("## ") {
                // End of properties section
                if let Some(property) = current_property.take() {
                    properties.push(property);
                }
                break;
            }
            
            if in_properties_section {
                if line.starts_with("Property ") && line.contains(":") {
                    // Save previous property
                    if let Some(property) = current_property.take() {
                        properties.push(property);
                    }
                    
                    // Start new property
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let property_id = parts[0].trim().replace(" ", "_").to_lowercase();
                        let property_name = parts[1].trim().to_string();
                        
                        current_property = Some(CorrectnessProperty {
                            id: property_id,
                            name: property_name,
                            description: String::new(),
                            property_type: PropertyType::Metamorphic,
                            requirements_refs: Vec::new(),
                        });
                    }
                }
                
                if let Some(ref mut property) = current_property {
                    if line.starts_with("*For any*") {
                        property.description = line.to_string();
                    }
                    
                    if line.starts_with("**Validates: Requirements") {
                        let refs_part = line.trim_start_matches("**Validates: Requirements")
                            .trim_end_matches("**")
                            .trim();
                        property.requirements_refs = refs_part.split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                    }
                }
            }
        }
        
        // Don't forget the last property
        if let Some(property) = current_property {
            properties.push(property);
        }
        
        properties
    }

    /// Validate correctness properties format
    pub fn validate_property_format(&self, property: &str) -> bool {
        property.contains("*For any*") && property.contains("**Validates: Requirements")
    }

    /// Validate design document structure
    pub fn validate_document(&self, content: &str) -> DesignValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        
        // Check for required sections
        let required_sections = [
            "## Overview",
            "## Architecture", 
            "## Components and Interfaces",
            "## Data Models",
            "## Correctness Properties",
            "## Error Handling",
            "## Testing Strategy"
        ];
        
        for section in required_sections {
            if !content.contains(section) {
                errors.push(format!("Missing required section: {}", section));
            }
        }
        
        // Validate correctness properties
        let properties = self.extract_correctness_properties(content);
        if properties.is_empty() {
            warnings.push("No correctness properties found".to_string());
        }
        
        for property in properties {
            if property.description.is_empty() {
                warnings.push(format!("Property '{}' missing description", property.name));
            }
            
            if property.requirements_refs.is_empty() {
                errors.push(format!("Property '{}' missing requirements references", property.name));
            }
        }
        
        DesignValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Check if design document exists
    pub fn exists(&self, feature_name: &str) -> bool {
        self.spec_root.join(feature_name).join("design.md").exists()
    }
}

/// Design document validation result
#[derive(Debug, Clone)]
pub struct DesignValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}