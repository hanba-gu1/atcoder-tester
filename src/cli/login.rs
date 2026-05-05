use std::{
    fs,
    io::{self, Write},
};

use anyhow::Result;

pub fn login() -> Result<()> {
    print!("enter REVEL_SESSION value: ");
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    let actester_dir = dirs::home_dir().unwrap().join(".actester");

    fs::create_dir_all(&actester_dir)?;
    fs::write(actester_dir.join("session_cookie"), buffer.trim())?;

    Ok(())
}
