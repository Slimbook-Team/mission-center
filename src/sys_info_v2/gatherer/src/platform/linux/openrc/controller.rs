use std::process::Command;
use std::sync::Arc;

use super::OpenRCError;

pub struct Controller;

impl Controller {
    pub fn enable_service(&self, service: &str) -> Result<(), OpenRCError> {
        let cmd = Command::new("pkexec")
            .arg("rc-update")
            .arg("add")
            .arg(service)
            .output()?;
        if !cmd.status.success() {
            let error_message =
                Arc::<str>::from(String::from_utf8_lossy(&cmd.stderr).as_ref().trim());
            return Err(OpenRCError::CommandExecutionError(
                error_message,
                cmd.status.code().unwrap_or(-1),
            ));
        }

        Ok(())
    }

    pub fn disable_service(&self, service: &str) -> Result<(), OpenRCError> {
        let cmd = Command::new("pkexec")
            .arg("rc-update")
            .arg("del")
            .arg(service)
            .output()?;
        if !cmd.status.success() {
            let error_message =
                Arc::<str>::from(String::from_utf8_lossy(&cmd.stderr).as_ref().trim());
            return Err(OpenRCError::CommandExecutionError(
                error_message,
                cmd.status.code().unwrap_or(-1),
            ));
        }

        Ok(())
    }

    pub fn start_service(&self, service: &str) -> Result<(), OpenRCError> {
        let cmd = Command::new("pkexec")
            .arg("rc-service")
            .arg(service)
            .arg("start")
            .output()?;
        if !cmd.status.success() {
            let error_message =
                Arc::<str>::from(String::from_utf8_lossy(&cmd.stderr).as_ref().trim());
            return Err(OpenRCError::CommandExecutionError(
                error_message,
                cmd.status.code().unwrap_or(-1),
            ));
        }

        Ok(())
    }

    pub fn stop_service(&self, service: &str) -> Result<(), OpenRCError> {
        let cmd = Command::new("pkexec")
            .arg("rc-service")
            .arg(service)
            .arg("stop")
            .output()?;
        if !cmd.status.success() {
            let error_message =
                Arc::<str>::from(String::from_utf8_lossy(&cmd.stderr).as_ref().trim());
            return Err(OpenRCError::CommandExecutionError(
                error_message,
                cmd.status.code().unwrap_or(-1),
            ));
        }

        Ok(())
    }

    pub fn restart_service(&self, service: &str) -> Result<(), OpenRCError> {
        let cmd = Command::new("pkexec")
            .arg("rc-service")
            .arg(service)
            .arg("restart")
            .output()?;
        if !cmd.status.success() {
            let error_message =
                Arc::<str>::from(String::from_utf8_lossy(&cmd.stderr).as_ref().trim());
            return Err(OpenRCError::CommandExecutionError(
                error_message,
                cmd.status.code().unwrap_or(-1),
            ));
        }

        Ok(())
    }
}
