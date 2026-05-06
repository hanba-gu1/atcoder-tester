use anyhow::Result;
use std::{fs, path::PathBuf};

#[derive(Debug, clap::Args)]
pub struct Init {
    #[arg(default_value = ".")]
    dest: PathBuf,
}

impl Init {
    pub fn init(&self) -> Result<()> {
        fs::write(
            self.dest.join(".gitignore"),
            include_bytes!("assets/gitignore"),
        )?;
        fs::write(
            self.dest.join("actester.toml"),
            include_bytes!("assets/actester.toml"),
        )?;
        fs::write(
            self.dest.join("Cargo.toml"),
            include_bytes!("assets/workspace-Cargo.toml"),
        )?;

        let cargo_dir = self.dest.join(".cargo/");
        fs::create_dir_all(&cargo_dir)?;
        fs::write(
            cargo_dir.join("config.toml"),
            include_bytes!("assets/cargo-config.toml"),
        )?;

        let libs_dir = self.dest.join("libs/");
        fs::create_dir_all(libs_dir.join("src"))?;
        fs::write(
            libs_dir.join("src/lib.rs"),
            include_bytes!("assets/libs-lib.rs"),
        )?;
        fs::write(
            libs_dir.join("Cargo.toml"),
            include_bytes!("assets/libs-Cargo.toml"),
        )?;

        let template_dir = self.dest.join("template/");
        fs::create_dir_all(template_dir.join("src"))?;
        fs::write(
            template_dir.join("src/main.rs"),
            include_bytes!("assets/template-main.rs"),
        )?;
        fs::write(
            template_dir.join("Cargo.toml"),
            include_bytes!("assets/template-Cargo.toml"),
        )?;

        Ok(())
    }
}
