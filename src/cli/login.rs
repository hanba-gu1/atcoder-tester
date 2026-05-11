use std::io::{self, Write};

use anyhow::Result;

use crate::api::http::set_session_token;

pub fn login() -> Result<()> {
    print!("enter REVEL_SESSION value: ");
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    set_session_token(&buffer)?;

    Ok(())
}
