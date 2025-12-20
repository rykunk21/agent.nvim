use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::spec::requirements::RequirementsManager;
use crate::spec::design::DesignManager;
use crate::spec::tasks::TasksManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecPhase {
    Requirements,
    Design,
    Tasks,
    Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecContext {
    pub id: String,
    pub feature_name: String,
    pub current_phase: SpecPhase,
    pub requirements: Option<String>,
    pub requirements_approved: bool,
    pub design: Option<String>,
    pub design_approved: bool,
    pub tasks: Option<String>,
    pub tasks_approved: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgress {
    pub current_phase: SpecPhase,
    pub requirements_exists: bool,
    pub requirements_approved: bool,
    pub design_exists: bool,
    pub design_approved: bool,
    pub tasks_exists: bool,
    pub tasks_approved: bool,
}

pub struct SpecEngine {
    specs: std::collections::HashMap<String, SpecContext>,
}

impl SpecEngine {
    pub fn new() -> Self {
        info!("Initializing spec engine");
        SpecEngine {
            specs: std::collections::HashMap::new(),
        }
    }

    /// Create a new spec
    pub fn create_spec(&mut self, feature_name: String) -> String {
        let spec_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        // Generate initial requirements template
        let requirements = RequirementsManager::create_requirements(&feature_name);

        let spec = SpecContext {
            id: spec_id.clone(),
            feature_name,
            current_phase: SpecPhase::Requirements,
            requirements: Some(requirements),
            requirements_approved: false,
            design: None,
            design_approved: false,
            tasks: None,
            tasks_approved: false,
            created_at: now,
            updated_at: now,
        };

        self.specs.insert(spec_id.clone(), spec);
        info!("Created spec: {}", spec_id);
        spec_id
    }

    /// Get a spec
    pub fn get_spec(&self, spec_id: &str) -> Option<&SpecContext> {
        self.specs.get(spec_id)
    }

    /// Get mutable spec
    pub fn get_spec_mut(&mut self, spec_id: &str) -> Option<&mut SpecContext> {
        self.specs.get_mut(spec_id)
    }

    /// Update requirements
    pub fn update_requirements(&mut self, spec_id: &str, requirements: String) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.requirements = Some(requirements);
        spec.updated_at = chrono::Utc::now();
        info!("Updated requirements for spec: {}", spec_id);
        Ok(())
    }

    /// Approve requirements
    pub fn approve_requirements(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.requirements_approved = true;
        spec.updated_at = chrono::Utc::now();
        info!("Approved requirements for spec: {}", spec_id);
        Ok(())
    }

    /// Generate and update design from requirements
    pub fn generate_design(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        if !spec.requirements_approved {
            return Err(anyhow::anyhow!("Requirements must be approved before generating design"));
        }

        if let Some(ref requirements) = spec.requirements {
            let design = DesignManager::generate_design_from_requirements(&spec.feature_name, requirements);
            spec.design = Some(design);
            spec.current_phase = SpecPhase::Design;
            spec.updated_at = chrono::Utc::now();
            info!("Generated design for spec: {}", spec_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Requirements not found for spec: {}", spec_id))
        }
    }

    /// Update design
    pub fn update_design(&mut self, spec_id: &str, design: String) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.design = Some(design);
        spec.current_phase = SpecPhase::Design;
        spec.updated_at = chrono::Utc::now();
        info!("Updated design for spec: {}", spec_id);
        Ok(())
    }

    /// Approve design
    pub fn approve_design(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.design_approved = true;
        spec.updated_at = chrono::Utc::now();
        info!("Approved design for spec: {}", spec_id);
        Ok(())
    }

    /// Generate and update tasks from design
    pub fn generate_tasks(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        if !spec.design_approved {
            return Err(anyhow::anyhow!("Design must be approved before generating tasks"));
        }

        if let Some(ref design) = spec.design {
            let tasks = TasksManager::generate_tasks_from_design(&spec.feature_name, design);
            spec.tasks = Some(tasks);
            spec.current_phase = SpecPhase::Tasks;
            spec.updated_at = chrono::Utc::now();
            info!("Generated tasks for spec: {}", spec_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Design not found for spec: {}", spec_id))
        }
    }

    /// Update tasks
    pub fn update_tasks(&mut self, spec_id: &str, tasks: String) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.tasks = Some(tasks);
        spec.current_phase = SpecPhase::Tasks;
        spec.updated_at = chrono::Utc::now();
        info!("Updated tasks for spec: {}", spec_id);
        Ok(())
    }

    /// Approve tasks
    pub fn approve_tasks(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.tasks_approved = true;
        spec.updated_at = chrono::Utc::now();
        info!("Approved tasks for spec: {}", spec_id);
        Ok(())
    }

    /// Advance to next phase
    pub fn advance_phase(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.current_phase = match spec.current_phase {
            SpecPhase::Requirements => {
                if !spec.requirements_approved {
                    return Err(anyhow::anyhow!("Requirements must be approved before advancing"));
                }
                SpecPhase::Design
            }
            SpecPhase::Design => {
                if !spec.design_approved {
                    return Err(anyhow::anyhow!("Design must be approved before advancing"));
                }
                SpecPhase::Tasks
            }
            SpecPhase::Tasks => {
                if !spec.tasks_approved {
                    return Err(anyhow::anyhow!("Tasks must be approved before advancing"));
                }
                SpecPhase::Implementation
            }
            SpecPhase::Implementation => SpecPhase::Implementation,
        };

        spec.updated_at = chrono::Utc::now();
        info!("Advanced spec {} to phase: {:?}", spec_id, spec.current_phase);
        Ok(())
    }

    /// Go to previous phase
    pub fn previous_phase(&mut self, spec_id: &str) -> Result<()> {
        let spec = self
            .specs
            .get_mut(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        spec.current_phase = match spec.current_phase {
            SpecPhase::Requirements => SpecPhase::Requirements,
            SpecPhase::Design => SpecPhase::Requirements,
            SpecPhase::Tasks => SpecPhase::Design,
            SpecPhase::Implementation => SpecPhase::Tasks,
        };

        spec.updated_at = chrono::Utc::now();
        info!("Moved spec {} to previous phase: {:?}", spec_id, spec.current_phase);
        Ok(())
    }

    /// Get workflow progress
    pub fn get_progress(&self, spec_id: &str) -> Result<WorkflowProgress> {
        let spec = self
            .specs
            .get(spec_id)
            .ok_or_else(|| anyhow::anyhow!("Spec not found: {}", spec_id))?;

        Ok(WorkflowProgress {
            current_phase: spec.current_phase.clone(),
            requirements_exists: spec.requirements.is_some(),
            requirements_approved: spec.requirements_approved,
            design_exists: spec.design.is_some(),
            design_approved: spec.design_approved,
            tasks_exists: spec.tasks.is_some(),
            tasks_approved: spec.tasks_approved,
        })
    }

    /// List all specs
    pub fn list_specs(&self) -> Vec<String> {
        self.specs.keys().cloned().collect()
    }

    /// Delete a spec
    pub fn delete_spec(&mut self, spec_id: &str) -> Result<()> {
        self.specs.remove(spec_id);
        info!("Deleted spec: {}", spec_id);
        Ok(())
    }
}

impl Default for SpecEngine {
    fn default() -> Self {
        Self::new()
    }
}
