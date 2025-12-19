use crate::agent::chat_manager::{SpecPhase, SpecContext};
use crate::spec::requirements::RequirementsManager;
use crate::spec::design::DesignManager;
use crate::spec::tasks::TasksManager;
use std::io::{Result, Error, ErrorKind};
use std::path::PathBuf;
use std::fs;

/// Manages spec-driven development workflow
pub struct SpecWorkflow {
    pub current_spec: Option<SpecContext>,
    spec_root: PathBuf,
    requirements_manager: RequirementsManager,
    design_manager: DesignManager,
    tasks_manager: TasksManager,
}

impl SpecWorkflow {
    pub fn new(spec_root: PathBuf) -> Self {
        let requirements_manager = RequirementsManager::new(spec_root.clone());
        let design_manager = DesignManager::new(spec_root.clone());
        let tasks_manager = TasksManager::new(spec_root.clone());
        
        SpecWorkflow {
            current_spec: None,
            spec_root,
            requirements_manager,
            design_manager,
            tasks_manager,
        }
    }

    /// Create a new spec workflow
    pub fn create_new_spec(&mut self, feature_name: String) -> Result<()> {
        // Validate feature name
        if feature_name.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Feature name cannot be empty"));
        }
        
        // Check if spec already exists
        if self.spec_exists(&feature_name) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("Spec already exists for feature: {}", feature_name)
            ));
        }
        
        // Create spec directory
        let spec_dir = self.spec_root.join(&feature_name);
        fs::create_dir_all(&spec_dir)?;
        
        // Initialize requirements document
        let requirements_content = self.requirements_manager.create_requirements(&feature_name)?;
        self.requirements_manager.save_requirements(&feature_name, &requirements_content)?;
        
        self.current_spec = Some(SpecContext {
            feature_name,
            current_phase: SpecPhase::Requirements,
            requirements: None, // Will be loaded when needed
            design: None,
            tasks: None,
        });

        Ok(())
    }

    /// Open an existing spec
    pub fn open_spec(&mut self, feature_name: String) -> Result<()> {
        if !self.spec_exists(&feature_name) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Spec not found for feature: {}", feature_name)
            ));
        }
        
        // Determine current phase based on existing documents
        let current_phase = self.determine_current_phase(&feature_name)?;
        
        // Load existing documents
        let requirements = if self.requirements_manager.exists(&feature_name) {
            Some(self.requirements_manager.load_requirements(&feature_name)?)
        } else {
            None
        };
        
        let design = if self.design_manager.exists(&feature_name) {
            Some(self.design_manager.load_design(&feature_name)?)
        } else {
            None
        };
        
        let tasks = if self.tasks_manager.exists(&feature_name) {
            Some(self.tasks_manager.load_tasks(&feature_name)?)
        } else {
            None
        };
        
        self.current_spec = Some(SpecContext {
            feature_name,
            current_phase,
            requirements,
            design,
            tasks,
        });

        Ok(())
    }

    /// Transition to the next phase
    pub fn next_phase(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            let next_phase = match spec.current_phase {
                SpecPhase::Requirements => {
                    // Validate requirements are approved before moving to design
                    if let Some(ref req) = spec.requirements {
                        if !req.approved {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "Requirements must be approved before proceeding to design"
                            ));
                        }
                    }
                    SpecPhase::Design
                }
                SpecPhase::Design => {
                    // Validate design is approved before moving to tasks
                    if let Some(ref design) = spec.design {
                        if !design.approved {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "Design must be approved before proceeding to tasks"
                            ));
                        }
                    }
                    SpecPhase::Tasks
                }
                SpecPhase::Tasks => {
                    // Validate tasks are approved before moving to implementation
                    if let Some(ref tasks) = spec.tasks {
                        if !tasks.approved {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                "Tasks must be approved before proceeding to implementation"
                            ));
                        }
                    }
                    SpecPhase::Implementation
                }
                SpecPhase::Implementation => SpecPhase::Implementation, // Stay in implementation
            };
            
            spec.current_phase = next_phase;
        }
        Ok(())
    }

    /// Go to previous phase
    pub fn previous_phase(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            spec.current_phase = match spec.current_phase {
                SpecPhase::Requirements => SpecPhase::Requirements, // Stay in requirements
                SpecPhase::Design => SpecPhase::Requirements,
                SpecPhase::Tasks => SpecPhase::Design,
                SpecPhase::Implementation => SpecPhase::Tasks,
            };
        }
        Ok(())
    }

    /// Jump to specific phase
    pub fn goto_phase(&mut self, phase: SpecPhase) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            spec.current_phase = phase;
        }
        Ok(())
    }

    /// Generate design document from current requirements
    pub fn generate_design(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            if let Some(ref requirements) = spec.requirements {
                let design_content = self.design_manager.generate_design_from_requirements(
                    &spec.feature_name,
                    &requirements.content
                )?;
                
                self.design_manager.save_design(&spec.feature_name, &design_content)?;
                
                // Reload design document
                spec.design = Some(self.design_manager.load_design(&spec.feature_name)?);
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Requirements must be loaded before generating design"
                ));
            }
        }
        Ok(())
    }

    /// Generate tasks document from current design
    pub fn generate_tasks(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            if let Some(ref design) = spec.design {
                let tasks_content = self.tasks_manager.generate_tasks_from_design(
                    &spec.feature_name,
                    &design.content
                )?;
                
                self.tasks_manager.save_tasks(&spec.feature_name, &tasks_content)?;
                
                // Reload tasks document
                spec.tasks = Some(self.tasks_manager.load_tasks(&spec.feature_name)?);
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Design must be loaded before generating tasks"
                ));
            }
        }
        Ok(())
    }

    /// Approve current phase document
    pub fn approve_current_phase(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            match spec.current_phase {
                SpecPhase::Requirements => {
                    if let Some(ref mut requirements) = spec.requirements {
                        requirements.approved = true;
                        // Save updated approval status
                        self.requirements_manager.update_requirements(&spec.feature_name, &requirements.content)?;
                    }
                }
                SpecPhase::Design => {
                    if let Some(ref mut design) = spec.design {
                        design.approved = true;
                        // Save updated approval status
                        self.design_manager.update_design(&spec.feature_name, &design.content)?;
                    }
                }
                SpecPhase::Tasks => {
                    if let Some(ref mut tasks) = spec.tasks {
                        tasks.approved = true;
                        // Save updated approval status
                        self.tasks_manager.update_tasks(&spec.feature_name, &tasks.content)?;
                    }
                }
                SpecPhase::Implementation => {
                    // Implementation phase doesn't have a document to approve
                }
            }
        }
        Ok(())
    }

    /// Get current spec context
    pub fn get_current_spec(&self) -> Option<&SpecContext> {
        self.current_spec.as_ref()
    }

    /// Get mutable current spec context
    pub fn get_current_spec_mut(&mut self) -> Option<&mut SpecContext> {
        self.current_spec.as_mut()
    }

    /// Check if a spec exists
    pub fn spec_exists(&self, feature_name: &str) -> bool {
        self.spec_root.join(feature_name).exists()
    }

    /// List all available specs
    pub fn list_specs(&self) -> Result<Vec<String>> {
        let mut specs = Vec::new();
        
        if !self.spec_root.exists() {
            return Ok(specs);
        }
        
        for entry in fs::read_dir(&self.spec_root)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    specs.push(name.to_string_lossy().to_string());
                }
            }
        }
        
        Ok(specs)
    }

    /// Determine current phase based on existing documents
    fn determine_current_phase(&self, feature_name: &str) -> Result<SpecPhase> {
        if self.tasks_manager.exists(feature_name) {
            Ok(SpecPhase::Tasks)
        } else if self.design_manager.exists(feature_name) {
            Ok(SpecPhase::Design)
        } else if self.requirements_manager.exists(feature_name) {
            Ok(SpecPhase::Requirements)
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("No spec documents found for feature: {}", feature_name)
            ))
        }
    }

    /// Get workflow progress information
    pub fn get_progress(&self) -> Option<WorkflowProgress> {
        if let Some(spec) = &self.current_spec {
            let requirements_exists = spec.requirements.is_some();
            let requirements_approved = spec.requirements.as_ref().map_or(false, |r| r.approved);
            
            let design_exists = spec.design.is_some();
            let design_approved = spec.design.as_ref().map_or(false, |d| d.approved);
            
            let tasks_exists = spec.tasks.is_some();
            let tasks_approved = spec.tasks.as_ref().map_or(false, |t| t.approved);
            
            Some(WorkflowProgress {
                current_phase: spec.current_phase.clone(),
                requirements_exists,
                requirements_approved,
                design_exists,
                design_approved,
                tasks_exists,
                tasks_approved,
            })
        } else {
            None
        }
    }

    /// Reload current spec documents from filesystem
    pub fn reload_current_spec(&mut self) -> Result<()> {
        if let Some(spec) = &mut self.current_spec {
            let feature_name = spec.feature_name.clone();
            
            // Reload requirements if they exist
            if self.requirements_manager.exists(&feature_name) {
                spec.requirements = Some(self.requirements_manager.load_requirements(&feature_name)?);
            }
            
            // Reload design if it exists
            if self.design_manager.exists(&feature_name) {
                spec.design = Some(self.design_manager.load_design(&feature_name)?);
            }
            
            // Reload tasks if they exist
            if self.tasks_manager.exists(&feature_name) {
                spec.tasks = Some(self.tasks_manager.load_tasks(&feature_name)?);
            }
        }
        Ok(())
    }

    /// Delete a spec and all its documents
    pub fn delete_spec(&mut self, feature_name: &str) -> Result<()> {
        let spec_dir = self.spec_root.join(feature_name);
        
        if spec_dir.exists() {
            fs::remove_dir_all(&spec_dir)?;
        }
        
        // Clear current spec if it's the one being deleted
        if let Some(ref current) = self.current_spec {
            if current.feature_name == feature_name {
                self.current_spec = None;
            }
        }
        
        Ok(())
    }
}

/// Workflow progress information
#[derive(Debug, Clone)]
pub struct WorkflowProgress {
    pub current_phase: SpecPhase,
    pub requirements_exists: bool,
    pub requirements_approved: bool,
    pub design_exists: bool,
    pub design_approved: bool,
    pub tasks_exists: bool,
    pub tasks_approved: bool,
}