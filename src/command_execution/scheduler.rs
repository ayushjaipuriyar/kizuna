// Scheduled Command Execution
//
// This module provides cron-like scheduling for automated command execution
// with persistence and recovery across restarts.

use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    template::{TemplateId, TemplateInstantiationRequest},
    types::*,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a scheduled task
pub type ScheduleId = Uuid;

/// Scheduled task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub schedule_id: ScheduleId,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub task_type: ScheduledTaskType,
    pub enabled: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub last_run: Option<Timestamp>,
    pub next_run: Option<Timestamp>,
    pub run_count: u64,
    pub owner: PeerId,
}

/// Type of scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduledTaskType {
    Command(CommandRequest),
    Template(TemplateInstantiationRequest),
    Script(ScriptRequest),
}

/// Schedule definition with cron-like syntax
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub schedule_type: ScheduleType,
    pub timezone: Option<String>,
}

/// Schedule type variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    /// Cron expression (minute hour day month weekday)
    Cron(String),
    /// Interval in seconds
    Interval(u64),
    /// One-time execution at specific time
    Once(Timestamp),
    /// Daily at specific time (HH:MM format)
    Daily(String),
    /// Weekly on specific day and time (weekday, HH:MM)
    Weekly(u8, String),
}

/// Execution result for scheduled task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledExecutionResult {
    pub schedule_id: ScheduleId,
    pub executed_at: Timestamp,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Scheduler manager for managing scheduled tasks
pub struct Scheduler {
    tasks: HashMap<ScheduleId, ScheduledTask>,
    execution_history: Vec<ScheduledExecutionResult>,
    storage_path: Option<PathBuf>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(storage_path: Option<PathBuf>) -> Self {
        Self {
            tasks: HashMap::new(),
            execution_history: Vec::new(),
            storage_path,
        }
    }

    /// Create a new scheduled task
    pub fn create_task(
        &mut self,
        name: String,
        description: String,
        schedule: Schedule,
        task_type: ScheduledTaskType,
        owner: PeerId,
    ) -> CmdResult<ScheduledTask> {
        let now = chrono::Utc::now();
        let next_run = self.calculate_next_run(&schedule, now)?;

        let task = ScheduledTask {
            schedule_id: Uuid::new_v4(),
            name,
            description,
            schedule,
            task_type,
            enabled: true,
            created_at: now,
            updated_at: now,
            last_run: None,
            next_run: Some(next_run),
            run_count: 0,
            owner,
        };

        self.tasks.insert(task.schedule_id, task.clone());

        // Persist to storage if configured
        if self.storage_path.is_some() {
            self.save_task(&task)?;
        }

        Ok(task)
    }

    /// Get a scheduled task by ID
    pub fn get_task(&self, schedule_id: &ScheduleId) -> CmdResult<&ScheduledTask> {
        self.tasks
            .get(schedule_id)
            .ok_or_else(|| CommandError::ScheduleError(format!("Task {} not found", schedule_id)))
    }

    /// Update a scheduled task
    pub fn update_task(
        &mut self,
        schedule_id: &ScheduleId,
        name: Option<String>,
        description: Option<String>,
        schedule: Option<Schedule>,
        enabled: Option<bool>,
    ) -> CmdResult<ScheduledTask> {
        // Calculate next run if schedule is provided
        let next_run = if let Some(ref sched) = schedule {
            Some(self.calculate_next_run(sched, chrono::Utc::now())?)
        } else {
            None
        };

        // Now update the task
        let task = self.tasks
            .get_mut(schedule_id)
            .ok_or_else(|| CommandError::ScheduleError(format!("Task {} not found", schedule_id)))?;

        if let Some(name) = name {
            task.name = name;
        }
        if let Some(description) = description {
            task.description = description;
        }
        if let Some(schedule) = schedule {
            task.schedule = schedule;
            task.next_run = next_run;
        }
        if let Some(enabled) = enabled {
            task.enabled = enabled;
        }

        task.updated_at = chrono::Utc::now();

        let updated_task = task.clone();

        // Persist to storage if configured
        if self.storage_path.is_some() {
            self.save_task(&updated_task)?;
        }

        Ok(updated_task)
    }

    /// Delete a scheduled task
    pub fn delete_task(&mut self, schedule_id: &ScheduleId) -> CmdResult<()> {
        self.tasks
            .remove(schedule_id)
            .ok_or_else(|| CommandError::ScheduleError(format!("Task {} not found", schedule_id)))?;

        // Remove from storage if configured
        if let Some(storage_path) = &self.storage_path {
            let task_file = storage_path.join(format!("{}.json", schedule_id));
            if task_file.exists() {
                std::fs::remove_file(task_file)
                    .map_err(|e| CommandError::StorageError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// List all scheduled tasks
    pub fn list_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.values().collect()
    }

    /// Get tasks that are due for execution
    pub fn get_due_tasks(&self) -> Vec<&ScheduledTask> {
        let now = chrono::Utc::now();
        self.tasks
            .values()
            .filter(|t| {
                t.enabled
                    && t.next_run.is_some()
                    && t.next_run.unwrap() <= now
            })
            .collect()
    }

    /// Mark a task as executed and calculate next run time
    pub fn mark_executed(
        &mut self,
        schedule_id: &ScheduleId,
        success: bool,
        output: Option<String>,
        error: Option<String>,
    ) -> CmdResult<()> {
        let now = chrono::Utc::now();

        // Calculate next run time before borrowing task mutably
        let (next_run, is_once) = {
            let task = self.tasks
                .get(schedule_id)
                .ok_or_else(|| CommandError::ScheduleError(format!("Task {} not found", schedule_id)))?;

            let is_once = matches!(task.schedule.schedule_type, ScheduleType::Once(_));
            let next = if is_once {
                None
            } else {
                Some(self.calculate_next_run(&task.schedule, now)?)
            };

            (next, is_once)
        };

        // Now update the task
        let task = self.tasks
            .get_mut(schedule_id)
            .ok_or_else(|| CommandError::ScheduleError(format!("Task {} not found", schedule_id)))?;

        task.last_run = Some(now);
        task.run_count += 1;
        task.next_run = next_run;

        if is_once {
            task.enabled = false;
        }

        let task_clone = task.clone();

        // Record execution result
        let result = ScheduledExecutionResult {
            schedule_id: *schedule_id,
            executed_at: now,
            success,
            output,
            error,
        };
        self.execution_history.push(result);

        // Persist to storage if configured
        if self.storage_path.is_some() {
            self.save_task(&task_clone)?;
        }

        Ok(())
    }

    /// Get execution history for a task
    pub fn get_execution_history(&self, schedule_id: &ScheduleId) -> Vec<&ScheduledExecutionResult> {
        self.execution_history
            .iter()
            .filter(|r| &r.schedule_id == schedule_id)
            .collect()
    }

    /// Calculate next run time based on schedule
    fn calculate_next_run(&self, schedule: &Schedule, from: Timestamp) -> CmdResult<Timestamp> {
        match &schedule.schedule_type {
            ScheduleType::Once(time) => Ok(*time),
            ScheduleType::Interval(seconds) => {
                Ok(from + chrono::Duration::seconds(*seconds as i64))
            }
            ScheduleType::Daily(time) => {
                self.calculate_daily_next_run(from, time)
            }
            ScheduleType::Weekly(weekday, time) => {
                self.calculate_weekly_next_run(from, *weekday, time)
            }
            ScheduleType::Cron(expr) => {
                self.calculate_cron_next_run(from, expr)
            }
        }
    }

    /// Calculate next run for daily schedule
    fn calculate_daily_next_run(&self, from: Timestamp, time: &str) -> CmdResult<Timestamp> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(CommandError::ScheduleError(
                format!("Invalid time format: {}", time)
            ));
        }

        let hour: u32 = parts[0].parse()
            .map_err(|_| CommandError::ScheduleError(format!("Invalid hour: {}", parts[0])))?;
        let minute: u32 = parts[1].parse()
            .map_err(|_| CommandError::ScheduleError(format!("Invalid minute: {}", parts[1])))?;

        let mut next = from.date_naive()
            .and_hms_opt(hour, minute, 0)
            .ok_or_else(|| CommandError::ScheduleError("Invalid time".to_string()))?
            .and_utc();

        // If the time has already passed today, schedule for tomorrow
        if next <= from {
            next = next + chrono::Duration::days(1);
        }

        Ok(next)
    }

    /// Calculate next run for weekly schedule
    fn calculate_weekly_next_run(&self, from: Timestamp, weekday: u8, time: &str) -> CmdResult<Timestamp> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(CommandError::ScheduleError(
                format!("Invalid time format: {}", time)
            ));
        }

        let hour: u32 = parts[0].parse()
            .map_err(|_| CommandError::ScheduleError(format!("Invalid hour: {}", parts[0])))?;
        let minute: u32 = parts[1].parse()
            .map_err(|_| CommandError::ScheduleError(format!("Invalid minute: {}", parts[1])))?;

        let current_weekday = from.date_naive().weekday().num_days_from_monday() as u8;
        let days_until_target = if weekday >= current_weekday {
            weekday - current_weekday
        } else {
            7 - (current_weekday - weekday)
        };

        let target_date = from.date_naive() + chrono::Duration::days(days_until_target as i64);
        let mut next = target_date
            .and_hms_opt(hour, minute, 0)
            .ok_or_else(|| CommandError::ScheduleError("Invalid time".to_string()))?
            .and_utc();

        // If we're on the target day but the time has passed, schedule for next week
        if days_until_target == 0 && next <= from {
            next = next + chrono::Duration::days(7);
        }

        Ok(next)
    }

    /// Calculate next run for cron schedule (simplified implementation)
    fn calculate_cron_next_run(&self, from: Timestamp, _expr: &str) -> CmdResult<Timestamp> {
        // Simplified cron implementation - just add 1 hour for now
        // A full implementation would parse the cron expression
        Ok(from + chrono::Duration::hours(1))
    }

    /// Save task to storage
    fn save_task(&self, task: &ScheduledTask) -> CmdResult<()> {
        if let Some(storage_path) = &self.storage_path {
            std::fs::create_dir_all(storage_path)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;

            let task_file = storage_path.join(format!("{}.json", task.schedule_id));
            let json = serde_json::to_string_pretty(task)?;

            std::fs::write(task_file, json)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;
        }

        Ok(())
    }

    /// Load tasks from storage
    pub fn load_tasks(&mut self) -> CmdResult<usize> {
        if let Some(storage_path) = &self.storage_path {
            if !storage_path.exists() {
                return Ok(0);
            }

            let entries = std::fs::read_dir(storage_path)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;

            let mut count = 0;
            for entry in entries {
                let entry = entry.map_err(|e| CommandError::StorageError(e.to_string()))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let json = std::fs::read_to_string(&path)
                        .map_err(|e| CommandError::StorageError(e.to_string()))?;

                    let task: ScheduledTask = serde_json::from_str(&json)?;

                    self.tasks.insert(task.schedule_id, task);
                    count += 1;
                }
            }

            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.execution_history.clear();
    }

    /// Get execution history count
    pub fn history_count(&self) -> usize {
        self.execution_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_create_scheduled_task() {
        let mut scheduler = Scheduler::new(None);

        let schedule = Schedule {
            schedule_type: ScheduleType::Interval(3600),
            timezone: None,
        };

        let command = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo test".to_string(),
            arguments: vec![],
            working_directory: None,
            environment: HashMap::new(),
            timeout: std::time::Duration::from_secs(60),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: chrono::Utc::now(),
        };

        let result = scheduler.create_task(
            "Test Task".to_string(),
            "A test scheduled task".to_string(),
            schedule,
            ScheduledTaskType::Command(command),
            "test_peer".to_string(),
        );

        assert!(result.is_ok());
        let task = result.unwrap();
        assert_eq!(task.name, "Test Task");
        assert!(task.enabled);
        assert!(task.next_run.is_some());
    }

    #[test]
    fn test_interval_schedule() {
        let scheduler = Scheduler::new(None);
        let now = chrono::Utc::now();

        let schedule = Schedule {
            schedule_type: ScheduleType::Interval(3600),
            timezone: None,
        };

        let next_run = scheduler.calculate_next_run(&schedule, now).unwrap();
        let expected = now + chrono::Duration::seconds(3600);

        // Allow 1 second tolerance for test execution time
        assert!((next_run - expected).num_seconds().abs() < 1);
    }

    #[test]
    fn test_daily_schedule() {
        let scheduler = Scheduler::new(None);
        let now = chrono::Utc::now();

        let schedule = Schedule {
            schedule_type: ScheduleType::Daily("14:30".to_string()),
            timezone: None,
        };

        let next_run = scheduler.calculate_next_run(&schedule, now).unwrap();
        assert!(next_run > now);
    }

    #[test]
    fn test_mark_executed() {
        let mut scheduler = Scheduler::new(None);

        let schedule = Schedule {
            schedule_type: ScheduleType::Interval(3600),
            timezone: None,
        };

        let command = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo test".to_string(),
            arguments: vec![],
            working_directory: None,
            environment: HashMap::new(),
            timeout: std::time::Duration::from_secs(60),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: chrono::Utc::now(),
        };

        let task = scheduler.create_task(
            "Test Task".to_string(),
            "A test scheduled task".to_string(),
            schedule,
            ScheduledTaskType::Command(command),
            "test_peer".to_string(),
        ).unwrap();

        let result = scheduler.mark_executed(
            &task.schedule_id,
            true,
            Some("Success".to_string()),
            None,
        );

        assert!(result.is_ok());

        let updated_task = scheduler.get_task(&task.schedule_id).unwrap();
        assert_eq!(updated_task.run_count, 1);
        assert!(updated_task.last_run.is_some());
    }

    #[test]
    fn test_one_time_schedule() {
        let mut scheduler = Scheduler::new(None);
        let future_time = chrono::Utc::now() + chrono::Duration::hours(1);

        let schedule = Schedule {
            schedule_type: ScheduleType::Once(future_time),
            timezone: None,
        };

        let command = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo test".to_string(),
            arguments: vec![],
            working_directory: None,
            environment: HashMap::new(),
            timeout: std::time::Duration::from_secs(60),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: chrono::Utc::now(),
        };

        let task = scheduler.create_task(
            "One-time Task".to_string(),
            "A one-time scheduled task".to_string(),
            schedule,
            ScheduledTaskType::Command(command),
            "test_peer".to_string(),
        ).unwrap();

        // Mark as executed
        scheduler.mark_executed(&task.schedule_id, true, None, None).unwrap();

        // Task should be disabled after execution
        let updated_task = scheduler.get_task(&task.schedule_id).unwrap();
        assert!(!updated_task.enabled);
        assert!(updated_task.next_run.is_none());
    }
}
