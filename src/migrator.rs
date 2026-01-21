use crate::DatabaseEngine;
use crate::adapters::{AppliedMigration, AsyncMigrationAdapter, MigrationAdapter};
use crate::package::metadata::{ChangeMetadata, EngineMetadata};
use std::time::Instant;

pub trait MigrationSource {
    fn engine_metadata(&mut self, engine: &DatabaseEngine) -> Result<EngineMetadata, Box<dyn std::error::Error>>;
    fn change_metadata(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<ChangeMetadata, Box<dyn std::error::Error>>;
    fn deploy_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<String, Box<dyn std::error::Error>>;
    fn verify_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn std::error::Error>>;
    fn revert_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn std::error::Error>>;
}

pub struct Migrator<'a, A, S: MigrationSource> {
    adapter: A,
    source: &'a mut S,
}

impl<'a, A: MigrationAdapter, S: MigrationSource> Migrator<'a, A, S> {
    pub fn new(adapter: A, source: &'a mut S) -> Self {
        Self { adapter, source }
    }

    pub fn apply(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.apply_to(None)
    }

    pub fn apply_to(&mut self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Ensuring history table exists");
        self.adapter.ensure_history_table()?;

        let engine = A::ENGINE;
        log::info!("Applying migrations for engine: {}", engine);
        let metadata = self.source.engine_metadata(&engine)?;
        let mut sequence = metadata.sequence().to_vec();

        if let Some(target) = target {
            log::info!("Targeting migration: {}", target);
            if let Some(pos) = sequence.iter().position(|s| s == target) {
                sequence.truncate(pos + 1);
            } else {
                return Err(format!("Target migration {} not found in sequence", target).into());
            }
        }

        log::debug!("Migration sequence: {:?}", sequence);

        for item in sequence {
            log::debug!("Checking migration: {}", item);
            let applied = self.adapter.get_applied_migration(&item)?;

            if let Some(record) = applied {
                if record.success {
                    // Check hash
                    if item.starts_with('@') {
                        log::debug!("Plan {} already applied successfully", item);
                        // Plan - Plans might not have hash in metadata or might have different verification logic
                        // For now let's assume if it succeeded it's fine
                        continue;
                    } else {
                        let change_metadata = self.source.change_metadata(&engine, &item)?;
                        if change_metadata.hash() != record.hash {
                            log::error!("Hash mismatch for change {}: expected {}, found {}", item, change_metadata.hash(), record.hash);
                            return Err(format!(
                                "Hash mismatch for change {}: expected {}, found {}",
                                item,
                                change_metadata.hash(),
                                record.hash
                            )
                            .into());
                        }
                        log::debug!("Change {} already applied and hash matches", item);
                        continue;
                    }
                } else {
                    log::warn!("Migration {} was previously attempted but failed. Retrying.", item);
                }
            }

            // Apply it
            let type_name = if item.starts_with('@') { "plan" } else { "change" };
            log::info!("Applying {} {}", type_name, item);
            let (hash, scripts) = if item.starts_with('@') {
                // Plans don't have scripts themselves, they are just markers in the sequence
                // for historical purposes. The changes they target are in the sequence.
                ("".to_string(), None)
            } else {
                let change_metadata = self.source.change_metadata(&engine, &item)?;
                let deploy = self.source.deploy_script(&engine, &item)?;
                let verify = self.source.verify_script(&engine, &item)?;
                (change_metadata.hash().to_string(), Some((deploy, verify)))
            };

            let start = Instant::now();
            let success = if let Some((deploy_sql, verify_sql)) = scripts {
                log::trace!("Executing deploy SQL for {}:\n{}", item, deploy_sql);
                match self.adapter.execute(&deploy_sql) {
                    Ok(_) => {
                        log::debug!("Successfully executed deploy for {}", item);
                        if let Some(verify_sql) = verify_sql {
                            log::trace!("Executing verify SQL for {}:\n{}", item, verify_sql);
                            match self.adapter.execute(&verify_sql) {
                                Ok(_) => {
                                    log::debug!("Successfully executed verify for {}", item);
                                    true
                                }
                                Err(e) => {
                                    log::error!("Error verifying {}: {}", item, e);
                                    false
                                }
                            }
                        } else {
                            true
                        }
                    }
                    Err(e) => {
                        log::error!("Error applying {}: {}", item, e);
                        false
                    }
                }
            } else {
                log::debug!("No script to execute for {}", item);
                true
            };
            let duration = start.elapsed();

            let migration_record = AppliedMigration {
                name: item.clone(),
                type_name: type_name.to_string(),
                hash,
                success,
            };

            log::debug!("Recording migration result for {} (success: {}, duration: {:?})", item, success, duration);
            self.adapter
                .record_migration(&migration_record, duration.as_millis() as u64)?;

            if !success {
                return Err(format!("Failed to apply migration {}", item).into());
            }
        }

        log::info!("Successfully applied all migrations for engine: {}", engine);
        Ok(())
    }

    pub fn revert(&mut self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Ensuring history table exists");
        self.adapter.ensure_history_table()?;

        let engine = A::ENGINE;
        log::info!("Reverting migrations for engine: {} to {:?}", engine, target);
        let metadata = self.source.engine_metadata(&engine)?;
        let sequence = metadata.sequence().to_vec();

        let to_revert = if let Some(target) = target {
            let target_pos = sequence.iter().position(|s| s == target).ok_or_else(|| {
                format!("Target migration {} not found in sequence", target)
            })?;
            sequence[(target_pos + 1)..].to_vec()
        } else {
            sequence
        };

        let mut to_revert = to_revert;
        to_revert.reverse();

        for item in to_revert {
            log::debug!("Checking if {} needs to be reverted", item);
            let applied = self.adapter.get_applied_migration(&item)?;

            if let Some(record) = applied {
                if record.success {
                    // Check if it's already been reverted in this session or previously
                    // Actually, let's just trust mark_reverted will handle it or if we want to be more careful:
                    // We should probably check if record.reverted_at is null in the adapter
                    
                    if item.starts_with('@') {
                        log::info!("Skipping plan {} (nothing to execute for revert)", item);
                        continue;
                    }

                    log::info!("Reverting change {}", item);
                    let revert_sql = self.source.revert_script(&engine, &item)?;

                    if let Some(sql) = revert_sql {
                        log::trace!("Executing revert SQL for {}:\n{}", item, sql);
                        self.adapter.execute(&sql)?;
                        log::debug!("Marking {} as reverted in history", item);
                        self.adapter.mark_reverted(&item)?;
                    } else {
                        log::warn!("No revert script found for {}, skipping SQL execution and NOT marking as reverted", item);
                    }
                }
            }
        }

        Ok(())
    }
}

impl<'a, A: AsyncMigrationAdapter, S: MigrationSource> Migrator<'a, A, S> {
    pub fn new_async(adapter: A, source: &'a mut S) -> Self {
        Self { adapter, source }
    }

    pub async fn apply_async(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.apply_to_async(None).await
    }

    pub async fn apply_to_async(&mut self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Ensuring history table exists (async)");
        self.adapter.ensure_history_table().await?;

        let engine = A::ENGINE;
        log::info!("Applying migrations for engine: {} (async)", engine);
        let metadata = self.source.engine_metadata(&engine)?;
        let mut sequence = metadata.sequence().to_vec();

        if let Some(target) = target {
            log::info!("Targeting migration: {} (async)", target);
            if let Some(pos) = sequence.iter().position(|s| s == target) {
                sequence.truncate(pos + 1);
            } else {
                return Err(format!("Target migration {} not found in sequence", target).into());
            }
        }

        log::debug!("Migration sequence: {:?}", sequence);

        for item in sequence {
            log::debug!("Checking migration: {}", item);
            let applied = self.adapter.get_applied_migration(&item).await?;

            if let Some(record) = applied {
                if record.success {
                    if item.starts_with('@') {
                        log::debug!("Plan {} already applied successfully", item);
                        continue;
                    } else {
                        let change_metadata = self.source.change_metadata(&engine, &item)?;
                        if change_metadata.hash() != record.hash {
                            log::error!("Hash mismatch for change {}: expected {}, found {}", item, change_metadata.hash(), record.hash);
                            return Err(format!(
                                "Hash mismatch for change {}: expected {}, found {}",
                                item,
                                change_metadata.hash(),
                                record.hash
                            )
                            .into());
                        }
                        log::debug!("Change {} already applied and hash matches", item);
                        continue;
                    }
                } else {
                    log::warn!("Migration {} was previously attempted but failed. Retrying.", item);
                }
            }

            let type_name = if item.starts_with('@') { "plan" } else { "change" };
            log::info!("Applying {} {}", type_name, item);
            let (hash, scripts) = if item.starts_with('@') {
                ("".to_string(), None)
            } else {
                let change_metadata = self.source.change_metadata(&engine, &item)?;
                let deploy = self.source.deploy_script(&engine, &item)?;
                let verify = self.source.verify_script(&engine, &item)?;
                (change_metadata.hash().to_string(), Some((deploy, verify)))
            };

            let start = Instant::now();
            let success = if let Some((deploy_sql, verify_sql)) = scripts {
                log::trace!("Executing deploy SQL for {}:\n{}", item, deploy_sql);
                match self.adapter.execute(&deploy_sql).await {
                    Ok(_) => {
                        log::debug!("Successfully executed deploy for {}", item);
                        if let Some(verify_sql) = verify_sql {
                            log::trace!("Executing verify SQL for {}:\n{}", item, verify_sql);
                            match self.adapter.execute(&verify_sql).await {
                                Ok(_) => {
                                    log::debug!("Successfully executed verify for {}", item);
                                    true
                                }
                                Err(e) => {
                                    log::error!("Error verifying {}: {}", item, e);
                                    false
                                }
                            }
                        } else {
                            true
                        }
                    }
                    Err(e) => {
                        log::error!("Error applying {}: {}", item, e);
                        false
                    }
                }
            } else {
                log::debug!("No script to execute for {}", item);
                true
            };
            let duration = start.elapsed();

            let migration_record = AppliedMigration {
                name: item.clone(),
                type_name: type_name.to_string(),
                hash,
                success,
            };

            log::debug!("Recording migration result for {} (success: {}, duration: {:?})", item, success, duration);
            self.adapter
                .record_migration(&migration_record, duration.as_millis() as u64)
                .await?;

            if !success {
                return Err(format!("Failed to apply migration {}", item).into());
            }
        }

        log::info!("Successfully applied all migrations for engine: {} (async)", engine);
        Ok(())
    }

    pub async fn revert_async(&mut self, target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Ensuring history table exists (async)");
        self.adapter.ensure_history_table().await?;

        let engine = A::ENGINE;
        log::info!("Reverting migrations for engine: {} to {:?} (async)", engine, target);
        let metadata = self.source.engine_metadata(&engine)?;
        let sequence = metadata.sequence().to_vec();

        let to_revert = if let Some(target) = target {
            let target_pos = sequence.iter().position(|s| s == target).ok_or_else(|| {
                format!("Target migration {} not found in sequence", target)
            })?;
            sequence[(target_pos + 1)..].to_vec()
        } else {
            sequence
        };

        let mut to_revert = to_revert;
        to_revert.reverse();

        for item in to_revert {
            log::debug!("Checking if {} needs to be reverted (async)", item);
            let applied = self.adapter.get_applied_migration(&item).await?;

            if let Some(record) = applied {
                if record.success {
                    if item.starts_with('@') {
                        log::info!("Skipping plan {} (nothing to execute for revert)", item);
                        continue;
                    }

                    log::info!("Reverting change {} (async)", item);
                    let revert_sql = self.source.revert_script(&engine, &item)?;

                    if let Some(sql) = revert_sql {
                        log::trace!("Executing revert SQL for {}:\n{}", item, sql);
                        self.adapter.execute(&sql).await?;
                        log::debug!("Marking {} as reverted in history (async)", item);
                        self.adapter.mark_reverted(&item).await?;
                    } else {
                        log::warn!("No revert script found for {}, skipping SQL execution and NOT marking as reverted (async)", item);
                    }
                }
            }
        }

        Ok(())
    }
}
