use serde_json;

#[derive(Default)]
pub struct Containerfile {
    instructions: Vec<String>,
}

impl Containerfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, image: &str) -> Self {
        self.instructions.push(format!("FROM {}", image));
        self
    }

    pub fn arg(mut self, key: &str, val: &str) -> Self {
        self.instructions.push(format!("ARG {}={}", key, val));
        self
    }

    pub fn run(mut self, cmd: &str) -> Self {
        self.instructions.push(format!("RUN {}", cmd));
        self
    }

    pub fn copy(mut self, src: &str, dst: &str) -> Self {
        self.instructions.push(format!("COPY {} {}", src, dst));
        self
    }

    pub fn env(mut self, key: &str, val: &str) -> Self {
        self.instructions.push(format!("ENV {}={}", key, val));
        self
    }

    pub fn expose(mut self, port: u16) -> Self {
        self.instructions.push(format!("EXPOSE {}", port));
        self
    }

    pub fn user(mut self, user: &str) -> Self {
        self.instructions.push(format!("USER {}", user));
        self
    }

    pub fn entrypoint(mut self, cmd: &[&str]) -> Self {
        let args = cmd
            .iter()
            .map(|s| serde_json::to_string(s).unwrap())
            .collect::<Vec<_>>()
            .join(", ");
        self.instructions.push(format!("ENTRYPOINT [{}]", args));
        self
    }

    pub fn extend(mut self, other_containerfile: &Self) -> Self {
        self.instructions
            .extend(other_containerfile.instructions.iter().cloned());
        self
    }

    pub fn build(self) -> String {
        self.instructions.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entrypoint_with_quotes() {
        let cf = Containerfile::new().entrypoint(&["echo", r#""hello""#]);
        assert_eq!(cf.build(), r#"ENTRYPOINT ["echo", "\"hello\""]"#);
    }
}
