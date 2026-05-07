use std::{
    io::{Write as _, stderr},
    path::Path,
    process::{self, Stdio},
};

use anyhow::{Context as _, Result};
use regex::Regex;

use crate::api::config::{Contest, Task};

pub fn sample_test(
    contest_dir: &Path,
    contest_data: &Contest,
    task: &Task,
    sample_number: usize,
    sample_in: &str,
    sample_out: &str,
) -> Result<bool> {
    let exec_file = contest_dir
        .parent()
        .unwrap()
        .join(format!("target/debug/{}-{}", contest_data.name, task.name));
    let mut child = process::Command::new(&exec_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run")?;
    child
        .stdin
        .as_mut()
        .context("failed to run")?
        .write_all(sample_in.as_ref())
        .context("failed to run")?;

    let output = child.wait_with_output().context("failed to run")?;

    let out_diveded: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect();
    let correct_divided: Vec<_> = sample_out.split_ascii_whitespace().collect();

    if output.status.success() {
        if out_diveded.len() == correct_divided.len()
            && out_diveded
                .iter()
                .zip(&correct_divided)
                .all(|(out, correct)| is_correct(out, correct))
        {
            eprintln!("Sample{sample_number} ... AC!");
            return Ok(true);
        } else {
            eprintln!("Sample{sample_number} ... WA!");
            eprintln!("Standard input:");
            eprintln!("{sample_in}");
            eprintln!("Standard output:");
            stderr().write_all(&output.stdout)?;
            eprintln!("Expected output:");
            eprintln!("{sample_out}");
        }
    } else {
        eprintln!("Sample{sample_number} ... RE!");
        if !output.stdout.is_empty() {
            eprintln!("Standard output");
            stderr().write_all(&output.stdout)?;
        }
        eprintln!("Standard error");
        stderr().write_all(&output.stderr)?;
    }

    Ok(false)
}

fn is_correct(out: &str, correct: &str) -> bool {
    if Regex::new(r#"^\d+\.\d+$"#).unwrap().is_match(out) {
        out.len() >= correct.len() && out[..correct.len() - 1] == correct[..correct.len() - 1]
    } else {
        out == correct
    }
}
