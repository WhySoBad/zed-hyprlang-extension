use std::{fs::create_dir_all, path::{Path, PathBuf}, process::{Command, Output}};

use anyhow::bail;

use crate::DEBUG;

pub struct Git {
    path: PathBuf
}

impl Git {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn command(&self, command: &str, args: &[&str]) -> anyhow::Result<Output> {
        if DEBUG.is_some() {
            println!("cargo::warning=running command: git {command} {}", args.join(" "));
        }

        let output = Command::new("git")
                .arg("--git-dir")
                .arg(self.path.join(".git"))
                .arg(command)
                .args(args)
                .current_dir(&self.path)
                .output()?;

        if DEBUG.is_some() {
            println!("cargo::warning=got output: {}", String::from_utf8_lossy(&output.stdout));
        }

        if output.status.success() {
            Ok(output)
        } else {
            bail!("{}", String::from_utf8_lossy(output.stderr.as_slice()));
        }
    }

    pub fn directory(&self) -> &Path {
        self.path.as_path()
    }

    /// Get all remotes of the repository
    pub fn remotes(&self) -> anyhow::Result<Vec<(String, String)>> {
        let output = self.command("remote", &["-v"])?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(
            stdout.lines().filter_map(|line| {
                let mut parts = line.split(|c: char| c.is_whitespace());
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            }).collect()
        )
    }

    /// Add a new remote to the repository
    pub fn add_remote(&self, name: &str, url: &str) -> anyhow::Result<()> {
        self.command("remote", &["add", name, url])?;
        Ok(())
    }

    /// Get all branches of the repository
    pub fn branches(&self) -> anyhow::Result<Vec<String>> {
        let output = self.command("branch", &["--format='%(refname:short)'"])?;
        Ok(String::from_utf8_lossy(&output.stdout).lines().map(|str| str.to_string()).collect())
    }

    /// Delete a specific branch from the repository
    pub fn delete_branch(&self, branch: &str) -> anyhow::Result<()> {
        self.command("branch", &["-D", branch])?;
        Ok(())
    }

    /// Checkout a (new) branch with a revision (optional)
    pub fn checkout_branch(&self, branch: &str, new: bool, revision: Option<&str>) -> anyhow::Result<()> {
        let mut args = vec![];
        if new {
            args.push("-b");
        }
        args.push(branch);
        if let Some(revision) = revision {
            args.push(revision);
        }
        self.command("checkout", &args)?;
        Ok(())
    }

    /// Check whether the repository has a branch which contains this name
    pub fn has_branch(&self, name: &str) -> bool {
        self.branches().is_ok_and(|branches| branches.iter().any(|b| b.contains(name)))
    }

    /// Fetch a remote with an revision (optional)
    pub fn fetch(&self, remote: &str, revision: Option<&str>) -> anyhow::Result<()> {
        let mut args = vec![remote];
        if let Some(revision) = revision {
            args.append(&mut vec!["--depth", "1", revision]);
        }
        self.command("fetch", &args)?;
        Ok(())
    }

    /// Initialize a new git repository
    pub fn init(&self) -> anyhow::Result<()> {
        if !self.path.exists() {
            create_dir_all(&self.path)?
        }

        self.command("init", &[])?;

        Ok(())
    }

    /// Apply a patch to the working tree
    pub fn apply(&self, patch_path: &str) -> anyhow::Result<()> {
        self.command("apply", &[patch_path])?;
        Ok(())
    }

    /// (Hard) reset the working tree
    pub fn reset(&self, hard: bool) -> anyhow::Result<()> {
        let mut args = vec![];
        if hard {
            args.push("--hard");
        }
        self.command("reset", &args)?;
        Ok(())
    }

    /// Remove untracked files and directories from the working tree
    pub fn clean(&self) -> anyhow::Result<()> {
        self.command("clean", &["-fd"])?;
        Ok(())
    }
}