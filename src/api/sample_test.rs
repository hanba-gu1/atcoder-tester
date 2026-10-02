use std::{
    fs,
    io::{Write as _, stderr},
    path::Path,
    process::{self, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, ensure};
use colored::Colorize;

use crate::api::config::{Contest, Task};

pub fn build_for_test(root_dir: &Path, contest_data: &Contest, task: &Task) -> Result<()> {
    let build_output = process::Command::new("cargo")
        .args([
            "build",
            "--package",
            &format!("{}-{}", contest_data.name, task.name),
        ])
        .current_dir(root_dir)
        .stderr(Stdio::inherit())
        .output()
        .context("failed to build")?;
    ensure!(build_output.status.success(), "falied to build");
    Ok(())
}

pub fn get_sample(task_dir: &Path, sample_number: usize) -> Result<Option<(String, String)>> {
    let sample_in_file = task_dir.join(format!("samples/{sample_number}.in"));
    let sample_out_file = task_dir.join(format!("samples/{sample_number}.out"));
    Ok(if sample_in_file.is_file() && sample_out_file.is_file() {
        let sample_in = fs::read_to_string(&sample_in_file)?;
        let sample_out = fs::read_to_string(&sample_out_file)?;
        Some((sample_in, sample_out))
    } else {
        None
    })
}

pub fn get_all_samples(task_dir: &Path) -> Result<Vec<(String, String)>> {
    let mut samples = Vec::new();
    for i in 1.. {
        if let Some(sample) = get_sample(task_dir, i)? {
            samples.push(sample);
        } else {
            break;
        }
    }
    Ok(samples)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResultType {
    Ac,
    Wa,
    Re,
    Tle,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TestResult {
    pub result_type: TestResultType,
    pub output: Output,
    pub exec_time: Duration,
}

fn is_correct(out: &str, correct: &str) -> bool {
    const DICIMAL_ERROR_MARGIN: f64 = 1e-6;

    if let (Ok(out), Ok(correct)) = (out.parse::<i64>(), correct.parse::<i64>()) {
        out == correct
    } else if let (Ok(out), Ok(correct)) = (out.parse::<f64>(), correct.parse::<f64>()) {
        let abs_error = (out - correct).abs();
        abs_error < DICIMAL_ERROR_MARGIN
            || (correct != 0.0 && abs_error / correct < DICIMAL_ERROR_MARGIN)
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

fn run_test(exec_file: &Path, input: &str) -> Result<(bool, Output, Duration)> {
    let mut child = process::Command::new(exec_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run")?;
    child
        .stdin
        .as_mut()
        .context("failed to run")?
        .write_all(input.as_ref())
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
            None => thread::sleep(Duration::from_millis(20)),
        }
    };
    let exec_time = start_time.elapsed();
    let output = child.wait_with_output()?;

    Ok((is_tle, output, exec_time))
}

pub fn sample_test(
    contest_dir: &Path,
    contest_data: &Contest,
    task: &Task,
    sample_in: &str,
    sample_out: &str,
) -> Result<TestResult> {
    let exec_file = contest_dir
        .parent()
        .unwrap()
        .join(format!("target/debug/{}-{}", contest_data.name, task.name));

    let (is_tle, output, exec_time) = run_test(&exec_file, sample_in)?;

    let result = if is_tle {
        TestResultType::Tle
    } else if !output.status.success() {
        TestResultType::Re
    } else if is_correct_all(&output.stdout, sample_out) {
        TestResultType::Ac
    } else {
        TestResultType::Wa
    };

    Ok(TestResult {
        result_type: result,
        output,
        exec_time,
    })
}

pub fn display_test_result(
    sample_number: usize,
    sample_in: &str,
    sample_out: &str,
    result: &TestResult,
) -> Result<()> {
    let result_text = match result.result_type {
        TestResultType::Ac => "AC".on_green(),
        TestResultType::Wa => "wA".on_yellow(),
        TestResultType::Re => "RE".on_yellow(),
        TestResultType::Tle => "TLE".on_yellow(),
    };

    eprintln!("-----------------------------------------");
    eprintln!("Sample{sample_number} ... {result_text}");
    eprintln!("Standard input:");
    eprintln!("{sample_in}");
    eprintln!("---------------");
    eprintln!("Standard output:");
    stderr().write_all(&result.output.stdout)?;
    eprintln!("---------------");
    eprintln!("Expected output:");
    eprintln!("{sample_out}");
    eprintln!("---------------");
    eprintln!("Standard error:");
    stderr().write_all(&result.output.stderr)?;
    eprintln!("-----------------------------------------");

    Ok(())
}

pub fn display_all_test_results(
    samples: &[(String, String)],
    results: &[TestResult],
) -> Result<()> {
    for (i, ((sample_in, sample_out), result)) in samples.iter().zip(results).enumerate() {
        display_test_result(i + 1, sample_in, sample_out, result)?;
    }
    Ok(())
}

pub fn test_all_sample(
    contest_dir: &Path,
    contest_data: &Contest,
    task: &Task,
    samples: &[(String, String)],
) -> Result<(bool, Vec<TestResult>)> {
    let mut ret = Vec::new();

    let mut all_ac = true;
    for (sample_in, sample_out) in samples {
        let result = sample_test(contest_dir, contest_data, task, sample_in, sample_out)?;
        all_ac &= result.result_type == TestResultType::Ac;
        ret.push(result);
    }
    Ok((all_ac, ret))
}
