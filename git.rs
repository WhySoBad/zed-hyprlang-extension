use std::{fs::create_dir_all, path::{Path, PathBuf}, process::{Command, Output}};

pub struct Git {
    path: PathBuf
}

impl Git {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn command(&self, command: &str, args: &[&str]) -> std::io::Result<Output> {
        Command::new("git")
                .arg("--git-dir")
                .arg(&self.path)
                .arg(command)
                .args(args)
                .current_dir(&self.path)
                .output()
    }

    pub fn directory(&self) -> &Path {
        self.path.as_path()
    }

    pub fn remotes(&self) -> std::io::Result<Vec<(String, String)>> {
        let output = self.command("remote", &["-v"])?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(
            stdout.lines().filter_map(|line| {
                let mut parts = line.split(|c: char| c.is_whitespace());
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            }).collect()
        )
    }

    pub fn add_remote(&self, name: &str, url: &str) -> std::io::Result<()> {
        self.command("remote", &["add", name, url])?;
        Ok(())
    }

    pub fn branches(&self) -> std::io::Result<Vec<String>> {
        let output = self.command("branch", &["--format='%(refname:short)'"])?;
        Ok(String::from_utf8_lossy(&output.stdout).lines().map(|str| str.to_string()).collect())
    }

    pub fn delete_branch(&self, branch: &str) -> std::io::Result<()> {
        self.command("branch", &["-D", branch])?;
        Ok(())
    }

    pub fn checkout_branch(&self, branch: &str, revision: Option<&str>) -> std::io::Result<()> {
        let mut args = vec!["-b", branch];
        if let Some(revision) = revision {
            args.push(revision);
        }
        self.command("checkout", &args)?;
        Ok(())
    }

    pub fn has_branch(&self, branch: &str) -> bool {
        let str = branch.to_string();
        self.branches().is_ok_and(|branches| branches.contains(&str))
    }

    pub fn fetch(&self, remote: &str, revision: Option<&str>) -> std::io::Result<()> {
        let mut args = vec!["fetch", remote];
        if let Some(revision) = revision {
            args.append(&mut vec!["--depth=1", revision]);
        }
        self.command("fetch", &args)?;
        Ok(())
    }

    pub fn u(&self) -> std::io::Result<()> {
        if !self.path.exists() {
            create_dir_all(&self.path)?
        }

        self.command("init", &[])?;

        Ok(())
    }
}