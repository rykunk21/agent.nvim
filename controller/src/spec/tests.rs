#[cfg(test)]
mod tests {
    use crate::spec::requirements::{RequirementsManager, EarsPattern, EarsValidation};
    use crate::spec::design::DesignManager;
    use crate::spec::tasks::{TasksManager, TaskStatus};
    use crate::spec::workflow::SpecWorkflow;
    use crate::agent::chat_manager::SpecPhase;
    use std::path::PathBuf;
    use tempfile::TempDir;
    
    fn create_test_spec_root() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let spec_root = temp_dir.path().to_path_buf();
        (temp_dir, spec_root)
    }

    #[test]
    fn test_requirements_manager_creation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = RequirementsManager::new(spec_root);
        
        // Test that manager is created successfully by checking it can create requirements
        let content = manager.create_requirements("test").unwrap();
        assert!(content.contains("Requirements Document"));
    }

    #[test]
    fn test_requirements_document_creation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = RequirementsManager::new(spec_root);
        
        let content = manager.create_requirements("test-feature").unwrap();
        
        assert!(content.contains("# Requirements Document"));
        assert!(content.contains("test-feature"));
        assert!(content.contains("## Introduction"));
        assert!(content.contains("## Glossary"));
        assert!(content.contains("## Requirements"));
    }

    #[test]
    fn test_ears_compliance_validation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = RequirementsManager::new(spec_root);
        
        // Valid EARS patterns
        assert!(manager.validate_ears_compliance("WHEN user clicks THEN THE system SHALL respond"));
        assert!(manager.validate_ears_compliance("WHILE condition holds THE system SHALL maintain state"));
        assert!(manager.validate_ears_compliance("IF error occurs THEN THE system SHALL handle gracefully"));
        assert!(manager.validate_ears_compliance("WHERE feature enabled THE system SHALL provide functionality"));
        assert!(manager.validate_ears_compliance("THE system SHALL always validate input"));
        
        // Invalid patterns
        assert!(!manager.validate_ears_compliance("The system should do something"));
        assert!(!manager.validate_ears_compliance("User wants feature"));
    }

    #[test]
    fn test_design_manager_creation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = DesignManager::new(spec_root);
        
        // Test that manager is created successfully by checking it can create design
        let content = manager.generate_design_template("test").unwrap();
        assert!(content.contains("Design Document"));
    }

    #[test]
    fn test_design_template_generation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = DesignManager::new(spec_root);
        
        let content = manager.generate_design_template("test-feature").unwrap();
        
        assert!(content.contains("# Design Document: test-feature"));
        assert!(content.contains("## Overview"));
        assert!(content.contains("## Architecture"));
        assert!(content.contains("## Components and Interfaces"));
        assert!(content.contains("## Data Models"));
        assert!(content.contains("## Correctness Properties"));
        assert!(content.contains("## Error Handling"));
        assert!(content.contains("## Testing Strategy"));
    }

    #[test]
    fn test_tasks_manager_creation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = TasksManager::new(spec_root);
        
        // Test that manager is created successfully by checking it can create tasks
        let content = manager.generate_tasks_template("test").unwrap();
        assert!(content.contains("Implementation Plan"));
    }

    #[test]
    fn test_tasks_template_generation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = TasksManager::new(spec_root);
        
        let content = manager.generate_tasks_template("test-feature").unwrap();
        
        assert!(content.contains("# Implementation Plan"));
        assert!(content.contains("- [ ] 1. Set up project structure"));
        assert!(content.contains("- [ ]* 1.1 Write property test"));
        assert!(content.contains("- [ ] 2. Implement core functionality"));
        assert!(content.contains("- [ ] 3. Checkpoint"));
    }

    #[test]
    fn test_task_status_parsing() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = TasksManager::new(spec_root);
        
        assert_eq!(manager.parse_task_status("- [ ] Task not started"), TaskStatus::NotStarted);
        assert_eq!(manager.parse_task_status("- [x] Task completed"), TaskStatus::Completed);
        assert_eq!(manager.parse_task_status("Some other text"), TaskStatus::Unknown);
    }

    #[test]
    fn test_spec_workflow_creation() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let workflow = SpecWorkflow::new(spec_root);
        
        assert!(workflow.current_spec.is_none());
        // Test that workflow can list specs (empty initially)
        let specs = workflow.list_specs().unwrap();
        assert_eq!(specs.len(), 0);
    }

    #[test]
    fn test_spec_workflow_new_spec() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let mut workflow = SpecWorkflow::new(spec_root);
        
        workflow.create_new_spec("test-feature".to_string()).unwrap();
        
        let spec = workflow.get_current_spec().unwrap();
        assert_eq!(spec.feature_name, "test-feature");
        assert_eq!(spec.current_phase, SpecPhase::Requirements);
    }

    #[test]
    fn test_spec_workflow_phase_transitions() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let mut workflow = SpecWorkflow::new(spec_root);
        
        workflow.create_new_spec("test-feature".to_string()).unwrap();
        
        // Should start in Requirements phase
        assert_eq!(workflow.get_current_spec().unwrap().current_phase, SpecPhase::Requirements);
        
        // Load the requirements that were created
        workflow.reload_current_spec().unwrap();
        
        // Should not be able to advance without approval
        assert!(workflow.next_phase().is_err());
        
        // Approve requirements and advance
        workflow.approve_current_phase().unwrap();
        workflow.next_phase().unwrap();
        assert_eq!(workflow.get_current_spec().unwrap().current_phase, SpecPhase::Design);
    }

    #[test]
    fn test_spec_workflow_list_specs() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let mut workflow = SpecWorkflow::new(spec_root);
        
        // Initially no specs
        let specs = workflow.list_specs().unwrap();
        assert_eq!(specs.len(), 0);
        
        // Create a spec
        workflow.create_new_spec("test-feature".to_string()).unwrap();
        
        // Should now have one spec
        let specs = workflow.list_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert!(specs.contains(&"test-feature".to_string()));
    }

    #[test]
    fn test_requirements_save_and_load() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = RequirementsManager::new(spec_root);
        
        let content = manager.create_requirements("test-feature").unwrap();
        let path = manager.save_requirements("test-feature", &content).unwrap();
        
        assert!(path.exists());
        
        let loaded = manager.load_requirements("test-feature").unwrap();
        assert_eq!(loaded.content, content);
        assert!(!loaded.approved); // Should default to not approved
    }

    #[test]
    fn test_design_save_and_load() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = DesignManager::new(spec_root);
        
        let content = manager.generate_design_template("test-feature").unwrap();
        let path = manager.save_design("test-feature", &content).unwrap();
        
        assert!(path.exists());
        
        let loaded = manager.load_design("test-feature").unwrap();
        assert_eq!(loaded.content, content);
        assert!(!loaded.approved);
    }

    #[test]
    fn test_tasks_save_and_load() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = TasksManager::new(spec_root);
        
        let content = manager.generate_tasks_template("test-feature").unwrap();
        let path = manager.save_tasks("test-feature", &content).unwrap();
        
        assert!(path.exists());
        
        let loaded = manager.load_tasks("test-feature").unwrap();
        assert_eq!(loaded.content, content);
        assert!(!loaded.approved);
    }

    #[test]
    fn test_workflow_progress_tracking() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let mut workflow = SpecWorkflow::new(spec_root);
        
        // No progress initially
        assert!(workflow.get_progress().is_none());
        
        // Create spec
        workflow.create_new_spec("test-feature".to_string()).unwrap();
        
        let progress = workflow.get_progress().unwrap();
        assert_eq!(progress.current_phase, SpecPhase::Requirements);
        assert!(!progress.requirements_exists);
        assert!(!progress.requirements_approved);
        assert!(!progress.design_exists);
        assert!(!progress.design_approved);
        assert!(!progress.tasks_exists);
        assert!(!progress.tasks_approved);
    }

    #[test]
    fn test_acceptance_criteria_extraction() {
        let (_temp_dir, spec_root) = create_test_spec_root();
        let manager = RequirementsManager::new(spec_root);
        
        let requirements_content = r#"
# Requirements Document

## Requirements

### Requirement 1

**User Story:** As a user, I want to login, so that I can access the system.

#### Acceptance Criteria

1. WHEN user enters valid credentials THEN the system SHALL authenticate the user
2. WHEN user enters invalid credentials THEN the system SHALL reject the login
3. WHILE user is authenticated THE system SHALL maintain session state

### Requirement 2

**User Story:** As an admin, I want to manage users, so that I can control access.

#### Acceptance Criteria

1. WHEN admin creates user THEN the system SHALL store user information
"#;
        
        let criteria = manager.extract_acceptance_criteria(requirements_content);
        
        assert_eq!(criteria.len(), 4);
        assert_eq!(criteria[0].id, "1.1");
        assert_eq!(criteria[1].id, "1.2");
        assert_eq!(criteria[2].id, "1.3");
        assert_eq!(criteria[3].id, "2.1");
        
        // Check EARS pattern validation
        assert!(matches!(criteria[0].ears_pattern, EarsValidation::Valid(EarsPattern::EventDriven)));
        assert!(matches!(criteria[2].ears_pattern, EarsValidation::Valid(EarsPattern::StateDriven)));
    }
}