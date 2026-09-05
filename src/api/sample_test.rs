use std::{
    io::{Write as _, stderr},
    path::Path,
    process::{self, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use colored::Colorize;
use lazy_regex::lazy_regex;

use crate::api::config::{Contest, Task};

#[derive(Debug, PartialEq, Eq)]
enum TestResult {
    Ac,
    Wa,
    Re,
    Tle,
}

fn is_correct(out: &str, correct: &str) -> bool {
    let decimal_pattern = lazy_regex!(r#"^\d+\.\d+$"#);

    if decimal_pattern.is_match(out) {
        let out: f64 = out.parse().unwrap();
        let correct: f64 = correct.parse().unwrap();
        (out - correct).abs() <= 1e-5
    } else {
        out == correct
    }
}

fn is_correct_all(out: &[u8], correct: &str) -> bool {
    let out_divided: Vec<_> = String::from_utf8_lossy(out)
        .into_owned()
        .split_ascii_whitespace()
        .map(str::to_string)
        .collect();
    let correct_divided: Vec<_> = correct.split_ascii_whitespace().collect();

    out_divided.len() == correct_divided.len()
        && out_divided
            .iter()
            .zip(&correct_divided)
            .all(|(out, correct)| is_correct(out, correct))
}

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

    let start_time = Instant::now();
    let timeout = Duration::from_secs(6);
    let is_tle = loop {
        match child.try_wait()? {
            Some(_) => break false,
            None if start_time.elapsed() >= timeout => {
                child.kill()?;
                break true;
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
    };
    let output = child.wait_with_output()?;

    let result = if is_tle {
        TestResult::Tle
    } else if !output.status.success() {
        TestResult::Re
    } else if is_correct_all(&output.stdout, sample_out) {
        TestResult::Ac
    } else {
        TestResult::Wa
    };

    let result_text = match result {
        TestResult::Ac => "AC".on_green(),
        TestResult::Wa => "wA".on_yellow(),
        TestResult::Re => "RE".on_yellow(),
        TestResult::Tle => "TLE".on_yellow(),
    };

    eprintln!("-----------------------------------------");
    eprintln!("Sample{sample_number} ... {result_text}");
    eprintln!("Standard input:");
    eprintln!("{sample_in}");
    eprintln!("---------------");
    eprintln!("Standard output:");
    stderr().write_all(&output.stdout)?;
    eprintln!("---------------");
    eprintln!("Expected output:");
    eprintln!("{sample_out}");
    eprintln!("---------------");
    eprintln!("Standard error:");
    stderr().write_all(&output.stderr)?;
    eprintln!("-----------------------------------------");

    Ok(result == TestResult::Ac)
}
