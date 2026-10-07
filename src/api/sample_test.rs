mod display;

use std::{
    fs,
    io::Write as _,
    path::Path,
    process::{self, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, anyhow};
use crossterm::style::Stylize as _;
use hooq::hooq;

use crate::api::config::{Contest, Task};
use display::Window;

#[hooq(anyhow)]
pub fn build_for_test(root_dir: &Path, contest_data: &Contest, task: &Task) -> Result<()> {
    let build_output = process::Command::new("cargo")
        .args([
            "build",
            "--package",
            &format!("{}-{}", contest_data.name, task.name),
        ])
        .current_dir(root_dir)
        .stderr(Stdio::inherit())
        .output()?;
    if build_output.status.success() {
        Ok(())
    } else {
        Err(anyhow!("falied to build"))
    }
}

#[hooq(anyhow)]
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

#[hooq(anyhow)]
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
pub enum TestStaus {
    Ac,
    Wa,
    Re,
    Tle,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TestResult {
    pub status: TestStaus,
    pub output: Output,
    pub exec_time: Duration,
}

fn is_correct(out: &str, correct: &str) -> bool {
    const DICIMAL_ERROR_MARGIN: f64 = 1e-6;

    if correct.contains('.')
        && let (Ok(out), Ok(correct)) = (out.parse::<f64>(), correct.parse::<f64>())
        && out.is_normal()
        && correct.is_normal()
    {
        let abs_error = (out - correct).abs();
        let rel_error = abs_error / correct.abs();
        abs_error.min(rel_error) < DICIMAL_ERROR_MARGIN
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

#[hooq(anyhow)]
fn run_test(exec_file: &Path, input: &str) -> Result<(bool, Output, Duration)> {
    let mut child = process::Command::new(exec_file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take()?.write_all(input.as_ref())?;

    let start_time = Instant::now();
    let timeout = Duration::from_secs(6);
    let is_tle = loop {
        match child.try_wait()? {
            Some(_) => break false,
            None if start_time.elapsed() >= timeout => {
                child.kill()?;
                break true;
            }
            None => thread::sleep(Duration::from_millis(1)),
        }
    };
    let exec_time = start_time.elapsed();
    let output = child.wait_with_output()?;

    Ok((is_tle, output, exec_time))
}

#[hooq(anyhow)]
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

    let status = if is_tle {
        TestStaus::Tle
    } else if !output.status.success() {
        TestStaus::Re
    } else if is_correct_all(&output.stdout, sample_out) {
        TestStaus::Ac
    } else {
        TestStaus::Wa
    };

    Ok(TestResult {
        status,
        output,
        exec_time,
    })
}

#[hooq(anyhow)]
pub fn test_all_sample(
    contest_dir: &Path,
    contest_data: &Contest,
    task: &Task,
    samples: &[(String, String)],
) -> Result<(bool, Vec<TestResult>)> {
    thread::scope(|s| -> Result<_> {
        let handles: Vec<_> = samples
            .iter()
            .map(|(sample_in, sample_out)| {
                let test =
                    move || sample_test(contest_dir, contest_data, task, sample_in, sample_out);
                s.spawn(test)
            })
            .collect();

        let mut ret = Vec::with_capacity(samples.len());
        let mut all_ac = true;
        for handle in handles {
            let result = handle.join().map_err(|err| anyhow!("{err:?}"))??;
            all_ac &= result.status == TestStaus::Ac;
            ret.push(result);
        }

        Ok((all_ac, ret))
    })
}

pub fn display_test_result(
    sample_number: usize,
    sample_in: &str,
    sample_out: &str,
    result: &TestResult,
) {
    let status_text = match result.status {
        TestStaus::Ac => " AC ".on_green().bold(),
        TestStaus::Wa => " WA ".on_yellow().bold(),
        TestStaus::Re => " RE ".on_yellow().bold(),
        TestStaus::Tle => " TLE ".on_yellow().bold(),
    };

    let width = terminal_size::terminal_size()
        .map(|(w, _)| (w.0 as usize * 2 / 2 - 1).min(127))
        .unwrap_or(127);

    eprintln!(
        "Sample{sample_number}    {status_text}   {} ms",
        result.exec_time.as_millis()
    );
    let expected_window = Window::new("Expected Output", sample_out);
    let stdin_window = Window::new("Standard Input", sample_in);
    Window::horizontal_print(&[expected_window, stdin_window], width);
    let stdout_window = Window::new(
        "Standard Output",
        String::from_utf8_lossy(&result.output.stdout),
    );
    if result.output.stderr.is_empty() {
        stdout_window.print(width);
    } else {
        let stderr_window = Window::new(
            "Standard Error",
            String::from_utf8_lossy(&result.output.stderr),
        );
        Window::horizontal_print(&[stdout_window, stderr_window], width);
    }
}

pub fn display_all_test_results(samples: &[(String, String)], results: &[TestResult]) {
    for (i, ((sample_in, sample_out), result)) in samples.iter().zip(results).enumerate() {
        display_test_result(i + 1, sample_in, sample_out, result);
    }
}

#[cfg(test)]
mod test {
    use crate::api::sample_test::is_correct;

    #[test]
    fn sample_correct_test() {
        let correct_pairs = [
            ("0", "0"),
            ("250", "250"),
            ("-300", "-300"),
            ("0.000001", "0.0000019"),
            ("-0.0000004", "0.0000004"),
            (
                "1000000000000000000000000000000000000000",
                "1000000000000000000000000000000000000000",
            ),
            ("10000005.0", "10000000.0"),
            ("ABC", "ABC"),
            ("!a^X++*];oewf^3", "!a^X++*];oewf^3"),
        ];
        for p in correct_pairs {
            assert!(is_correct(p.0, p.1), "{p:?}");
        }
    }
    #[test]
    fn sample_incorrect_test() {
        let incorrect_pairs = [
            ("0", "1"),
            ("250", "249"),
            ("-301", "-300"),
            ("0.000001", "0.000002"),
            ("-0.0000005", "0.0000005"),
            (
                "1000000000000000000000000000000000000000",
                "1000000000000000000000000000000000000001",
            ),
            ("10000010.0", "10000000.0"),
            ("ABC", "ABD"),
            ("!a^X+++];oewf^3", "!a^X++*];oewf^3"),
        ];
        for p in incorrect_pairs {
            assert!(!is_correct(p.0, p.1), "{p:?}");
        }
    }
}
