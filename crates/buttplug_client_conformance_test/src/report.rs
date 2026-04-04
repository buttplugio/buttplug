// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::step::SequenceResult;
use serde::Serialize;

/// Collects and formats test results for output
#[derive(Clone, Serialize)]
pub struct Report {
  pub sequences: Vec<SequenceResult>,
}

impl Report {
  /// Create a new empty report
  pub fn new() -> Self {
    Report {
      sequences: Vec::new(),
    }
  }

  /// Add a sequence result to the report
  pub fn add_result(&mut self, result: SequenceResult) {
    self.sequences.push(result);
  }

  /// Check if all sequences passed
  pub fn all_passed(&self) -> bool {
    self.sequences.iter().all(|seq| seq.passed)
  }

  /// Format the report as human-readable stdout
  pub fn format_stdout(&self) -> String {
    let mut output = String::new();

    for sequence in &self.sequences {
      output.push_str(&format!("=== Sequence: {} ===\n", sequence.sequence_name));

      let mut passed_count = 0;
      for step in &sequence.steps {
        let status = if step.passed { "[PASS]" } else { "[FAIL]" };
        output.push_str(&format!(
          "  {} {} ({}ms)\n",
          status, step.step_name, step.duration_ms
        ));

        if !step.passed {
          if let Some(error) = &step.error {
            output.push_str(&format!("         Error: {}\n", error));
          }
        } else {
          passed_count += 1;
        }
      }

      let total = sequence.steps.len();
      output.push_str(&format!("\nResults: {}/{} passed\n\n", passed_count, total));
    }

    output
  }

  /// Format the report as structured JSON
  pub fn format_json(&self) -> String {
    serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
  }
}

impl Default for Report {
  fn default() -> Self {
    Self::new()
  }
}
