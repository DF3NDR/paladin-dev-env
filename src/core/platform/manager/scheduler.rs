/*
Scheduler

This module is responsible for running the scheduler. The scheduler is a background
service that manages and executes jobs at specified intervals or times.
*/

use crate::core::platform::container::job::Job;
use crate::core::platform::container::task::{Task, DataBackupService, ContentIndexingService};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;
use chrono::{DateTime, Utc, Datelike};

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub job: Job,
    pub schedule: Schedule,
    pub enabled: bool,
    pub next_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum Schedule {
    Interval(Duration),
    Daily(u32, u32), // hour, minute
    Weekly(u8, u32, u32), // day of week (0=Sunday), hour, minute
    Monthly(u32, u32, u32), // day of month, hour, minute
    Once(DateTime<Utc>),
}

pub struct Scheduler {
    scheduled_jobs: HashMap<Uuid, ScheduledJob>,
    running: bool,
    tick_interval: Duration,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            scheduled_jobs: HashMap::new(),
            running: false,
            tick_interval: Duration::from_secs(60), // Check every minute
        }
    }

    pub fn add_job(&mut self, job: Job, schedule: Schedule) -> Uuid {
        let job_id = job.id();
        let scheduled_job = ScheduledJob {
            job,
            schedule: schedule.clone(),
            enabled: true,
            next_run: Self::calculate_next_run(&schedule),
        };
        
        self.scheduled_jobs.insert(job_id, scheduled_job);
        println!("Added job {} to scheduler", job_id);
        job_id
    }

    pub fn remove_job(&mut self, job_id: Uuid) -> bool {
        self.scheduled_jobs.remove(&job_id).is_some()
    }

    pub fn enable_job(&mut self, job_id: Uuid) -> bool {
        if let Some(scheduled_job) = self.scheduled_jobs.get_mut(&job_id) {
            scheduled_job.enabled = true;
            scheduled_job.next_run = Self::calculate_next_run(&scheduled_job.schedule);
            true
        } else {
            false
        }
    }

    pub fn disable_job(&mut self, job_id: Uuid) -> bool {
        if let Some(scheduled_job) = self.scheduled_jobs.get_mut(&job_id) {
            scheduled_job.enabled = false;
            scheduled_job.next_run = None;
            true
        } else {
            false
        }
    }

    pub async fn start(&mut self) {
        if self.running {
            println!("Scheduler is already running");
            return;
        }

        self.running = true;
        println!("Starting scheduler with {} jobs", self.scheduled_jobs.len());

        // Add some default jobs
        self.add_default_jobs();

        let mut interval = interval(self.tick_interval);

        while self.running {
            interval.tick().await;
            self.check_and_execute_jobs().await;
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
        println!("Scheduler stopped");
    }

    async fn check_and_execute_jobs(&mut self) {
        let now = Utc::now();
        let mut jobs_to_execute = Vec::new();

        // Collect jobs that need to be executed
        for (job_id, scheduled_job) in &self.scheduled_jobs {
            if scheduled_job.enabled {
                if let Some(next_run) = scheduled_job.next_run {
                    if now >= next_run {
                        jobs_to_execute.push(*job_id);
                    }
                }
            }
        }

        // Execute collected jobs
        for job_id in jobs_to_execute {
            if let Some(scheduled_job) = self.scheduled_jobs.get_mut(&job_id) {
                println!("Executing scheduled job: {}", scheduled_job.job.name);
                
                match scheduled_job.job.execute().await {
                    Ok(results) => {
                        let successful = results.iter().filter(|r| r.is_ok()).count();
                        let failed = results.len() - successful;
                        println!("Job '{}' completed: {} successful, {} failed", 
                                scheduled_job.job.name, successful, failed);
                    },
                    Err(e) => {
                        println!("Job '{}' failed: {}", scheduled_job.job.name, e);
                    }
                }

                // Update next run time
                scheduled_job.next_run = Self::calculate_next_run(&scheduled_job.schedule);
                if let Some(next_run) = scheduled_job.next_run {
                    println!("Next run for job '{}': {}", scheduled_job.job.name, next_run);
                }
            }
        }
    }

    fn add_default_jobs(&mut self) {
        // Data backup job - runs every 6 hours
        let backup_task = Task::new(
            "Daily Backup".to_string(),
            Box::new(DataBackupService {
                backup_path: "/backup".to_string(),
            }),
        );
        let backup_job = Job::new("Daily Backup Job".to_string(), vec![backup_task]);
        self.add_job(backup_job, Schedule::Interval(Duration::from_secs(6 * 3600)));

        // Content indexing job - runs every hour
        let indexing_task = Task::new(
            "Content Indexing".to_string(),
            Box::new(ContentIndexingService),
        );
        let indexing_job = Job::new("Content Indexing Job".to_string(), vec![indexing_task]);
        self.add_job(indexing_job, Schedule::Interval(Duration::from_secs(3600)));
    }

    fn calculate_next_run(schedule: &Schedule) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        
        match schedule {
            Schedule::Interval(duration) => {
                Some(now + chrono::Duration::from_std(*duration).ok()?)
            },
            Schedule::Daily(hour, minute) => {
                let mut next = now.date_naive().and_hms_opt(*hour, *minute, 0)?;
                if next <= now.naive_utc() {
                    next = next + chrono::Duration::days(1);
                }
                Some(DateTime::from_naive_utc_and_offset(next, Utc))
            },
            Schedule::Weekly(day_of_week, hour, minute) => {
                // Implementation for weekly scheduling
                let current_weekday = now.weekday().number_from_sunday() as u8;
                let days_until_target = if *day_of_week >= current_weekday {
                    *day_of_week - current_weekday
                } else {
                    7 - current_weekday + *day_of_week
                };
                
                let target_date = now.date_naive() + chrono::Duration::days(days_until_target as i64);
                let next = target_date.and_hms_opt(*hour, *minute, 0)?;
                Some(DateTime::from_naive_utc_and_offset(next, Utc))
            },
            Schedule::Monthly(day, hour, minute) => {
                // Implementation for monthly scheduling
                let next = now.date_naive()
                    .with_day(*day)?
                    .and_hms_opt(*hour, *minute, 0)?;
                Some(DateTime::from_naive_utc_and_offset(next, Utc))
            },
            Schedule::Once(datetime) => {
                if *datetime > now {
                    Some(*datetime)
                } else {
                    None // One-time job already passed
                }
            },
        }
    }

    pub fn list_jobs(&self) -> Vec<(Uuid, &str, bool, Option<DateTime<Utc>>)> {
        self.scheduled_jobs
            .iter()
            .map(|(id, scheduled_job)| {
                (*id, scheduled_job.job.name.as_str(), scheduled_job.enabled, scheduled_job.next_run)
            })
            .collect()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn start_scheduler() {
    let mut scheduler = Scheduler::new();
    scheduler.start().await;
}