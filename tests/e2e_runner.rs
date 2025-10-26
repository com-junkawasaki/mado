// Copyright 2024 Soft KVM Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! E2E Test Runner
//!
//! This module provides integration with Playwright for comprehensive
//! end-to-end testing of the Soft KVM application.

use std::process::Command;
use std::env;

/// Run Playwright E2E tests
pub fn run_e2e_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting E2E tests...");

    // Check if Playwright is installed
    if !is_playwright_installed() {
        println!("Installing Playwright...");
        install_playwright()?;
    }

    // Run the tests
    let mut cmd = Command::new("npx");
    cmd.args(["playwright", "test"])
       .current_dir(env::current_dir()?);

    // Add environment variables for testing
    cmd.env("NODE_ENV", "test");

    let status = cmd.status()?;

    if status.success() {
        println!("E2E tests passed!");
        Ok(())
    } else {
        println!("E2E tests failed!");
        std::process::exit(1);
    }
}

/// Run E2E tests with UI
pub fn run_e2e_tests_with_ui() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting E2E tests with UI...");

    if !is_playwright_installed() {
        install_playwright()?;
    }

    let mut cmd = Command::new("npx");
    cmd.args(["playwright", "test", "--ui"])
       .current_dir(env::current_dir()?);

    cmd.env("NODE_ENV", "test");

    let status = cmd.status()?;

    if status.success() {
        println!("E2E UI tests passed!");
        Ok(())
    } else {
        println!("E2E UI tests failed!");
        std::process::exit(1);
    }
}

/// Check if Playwright is installed
fn is_playwright_installed() -> bool {
    Command::new("npx")
        .args(["playwright", "--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install Playwright
fn install_playwright() -> Result<(), Box<dyn std::error::Error>> {
    println!("Installing Playwright...");

    let status = Command::new("npm")
        .args(["install"])
        .current_dir(env::current_dir()?)
        .status()?;

    if !status.success() {
        return Err("Failed to install Playwright".into());
    }

    // Install Playwright browsers
    let status = Command::new("npx")
        .args(["playwright", "install"])
        .current_dir(env::current_dir()?)
        .status()?;

    if !status.success() {
        return Err("Failed to install Playwright browsers".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playwright_installation_check() {
        // This test just checks that the function doesn't panic
        let _ = is_playwright_installed();
    }

    #[test]
    #[ignore] // Requires actual Playwright installation
    fn test_playwright_installation() {
        let result = install_playwright();
        // This might fail in CI, so we just check it doesn't panic
        let _ = result;
    }
}
