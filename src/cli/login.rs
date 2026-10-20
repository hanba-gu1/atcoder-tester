use std::io::{self, Write};

use anyhow::Result;
use hooq::hooq;

use crate::api::http::set_session_token;

#[hooq(anyhow)]
pub fn login() -> Result<()> {
    print!("enter REVEL_SESSION value: ");
    io::stdout().flush()?;

    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    set_session_token(&buffer)?;

    Ok(())
}
