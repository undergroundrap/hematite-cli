use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let template = args
        .get("template")
        .and_then(|v| v.as_str())
        .ok_or("template_gen: 'template' is required")?;

    // Handle list action
    if template == "list" {
        return Ok(list_templates());
    }

    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let output_path = args.get("output").and_then(|v| v.as_str());
    let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);

    // Collect substitution variables from args
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            if matches!(k.as_str(), "template" | "output" | "dry_run" | "_root") {
                continue;
            }
            if let Some(s) = v.as_str() {
                vars.insert(k.clone(), s.to_string());
            }
        }
    }

    let (default_path, content) = render_template(template, &vars)?;
    let dest_name = output_path.unwrap_or(default_path);
    let dest = root.join(dest_name);

    if dry_run {
        return Ok(format!(
            "template_gen [DRY RUN]: would write to {}\n\n{content}",
            dest.display()
        ));
    }

    // Check if file already exists
    if dest.exists() {
        return Err(format!(
            "template_gen: '{}' already exists. Pass output='path/to/file' to write elsewhere, \
             or rename the existing file first.",
            dest.display()
        ));
    }

    // Create parent dirs
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("template_gen: failed to create directory: {e}"))?;
        }
    }

    std::fs::write(&dest, &content)
        .map_err(|e| format!("template_gen: failed to write '{}': {e}", dest.display()))?;

    Ok(format!(
        "template_gen: wrote {} ({} bytes)\n\nContent preview (first 400 chars):\n{}",
        dest.display(),
        content.len(),
        &content[..content.len().min(400)]
    ))
}

fn list_templates() -> String {
    let mut out = "AVAILABLE TEMPLATES\n".to_string();
    out.push_str(&"─".repeat(60));
    out.push('\n');
    let templates: &[(&str, &str)] = &[
        ("dockerfile-node",       "Dockerfile for a Node.js application"),
        ("dockerfile-python",     "Dockerfile for a Python application"),
        ("dockerfile-rust",       "Dockerfile for a Rust application (multi-stage)"),
        ("dockerfile-go",         "Dockerfile for a Go application (multi-stage)"),
        ("ci-github-node",        ".github/workflows/ci.yml for Node.js (GitHub Actions)"),
        ("ci-github-python",      ".github/workflows/ci.yml for Python (GitHub Actions)"),
        ("ci-github-rust",        ".github/workflows/ci.yml for Rust (GitHub Actions)"),
        ("gitignore-node",        ".gitignore for Node.js projects"),
        ("gitignore-python",      ".gitignore for Python projects"),
        ("gitignore-rust",        ".gitignore for Rust projects"),
        ("gitignore-general",     ".gitignore with common OS and editor exclusions"),
        ("env-template",          ".env.example with common variable stubs"),
        ("makefile-node",         "Makefile with common Node.js targets"),
        ("makefile-python",       "Makefile with common Python targets"),
        ("makefile-rust",         "Makefile with common Rust targets"),
        ("docker-compose",        "docker-compose.yml with web + db + redis services"),
        ("pre-commit",            ".pre-commit-config.yaml with common hooks"),
        ("editorconfig",          ".editorconfig for consistent formatting"),
        ("dependabot",            ".github/dependabot.yml for automated dependency updates"),
        ("codeowners",            ".github/CODEOWNERS template"),
        ("pr-template",           ".github/pull_request_template.md"),
        ("issue-bug",             ".github/ISSUE_TEMPLATE/bug_report.md"),
        ("issue-feature",         ".github/ISSUE_TEMPLATE/feature_request.md"),
    ];
    for (name, desc) in templates {
        out.push_str(&format!("  {:30}  {desc}\n", name));
    }
    out.push_str("\nUsage: template_gen(template: \"dockerfile-rust\") or template_gen(template: \"ci-github-node\", project_name: \"my-app\")");
    out
}

fn render_template(name: &str, vars: &BTreeMap<String, String>) -> Result<(&'static str, String), String> {
    let project = vars.get("project_name").map(|s| s.as_str()).unwrap_or("my-app");
    let port = vars.get("port").map(|s| s.as_str()).unwrap_or("3000");
    let python_ver = vars.get("python_version").map(|s| s.as_str()).unwrap_or("3.12");
    let node_ver = vars.get("node_version").map(|s| s.as_str()).unwrap_or("20");
    let rust_ver = vars.get("rust_version").map(|s| s.as_str()).unwrap_or("1.82");
    let go_ver = vars.get("go_version").map(|s| s.as_str()).unwrap_or("1.23");
    let _registry = vars.get("registry").map(|s| s.as_str()).unwrap_or("ghcr.io/your-org");

    match name {
        "dockerfile-node" => Ok(("Dockerfile", format!(r#"FROM node:{node_ver}-alpine AS deps
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

FROM node:{node_ver}-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
COPY --from=deps /app/node_modules ./node_modules
COPY . .
EXPOSE {port}
CMD ["node", "src/index.js"]
"#))),

        "dockerfile-python" => Ok(("Dockerfile", format!(r#"FROM python:{python_ver}-slim
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE {port}
CMD ["python", "main.py"]
"#))),

        "dockerfile-rust" => Ok(("Dockerfile", format!(r#"FROM rust:{rust_ver}-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {{}}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runner
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/{project} /usr/local/bin/{project}
EXPOSE {port}
CMD ["{project}"]
"#, project=project, port=port, rust_ver=rust_ver))),

        "dockerfile-go" => Ok(("Dockerfile", format!(r#"FROM golang:{go_ver}-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /app/{project} .

FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/{project} /{project}
EXPOSE {port}
ENTRYPOINT ["/{project}"]
"#, project=project, port=port, go_ver=go_ver))),

        "ci-github-node" => Ok((".github/workflows/ci.yml", format!(r#"name: CI

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '{node_ver}'
          cache: 'npm'
      - run: npm ci
      - run: npm test
      - run: npm run lint --if-present
      - run: npm run build --if-present
"#))),

        "ci-github-python" => Ok((".github/workflows/ci.yml", format!(r#"name: CI

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '{python_ver}'
          cache: 'pip'
      - run: pip install -r requirements.txt
      - run: pip install pytest ruff mypy
      - run: ruff check .
      - run: pytest
"#))),

        "ci-github-rust" => Ok((".github/workflows/ci.yml", format!(r#"name: CI

on:
  push:
    branches: [main, master]
  pull_request:
    branches: [main, master]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test

  build-release:
    runs-on: ubuntu-latest
    needs: test
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release
"#))),

        "gitignore-node" => Ok((".gitignore", r#"node_modules/
dist/
build/
.next/
.nuxt/
coverage/
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*
.env
.env.local
.env.*.local
.DS_Store
*.tsbuildinfo
"#.to_string())),

        "gitignore-python" => Ok((".gitignore", r#"__pycache__/
*.pyc
*.pyo
*.pyd
.Python
.venv/
venv/
ENV/
env/
.env
dist/
build/
*.egg-info/
.eggs/
*.egg
.pytest_cache/
.mypy_cache/
.ruff_cache/
coverage.xml
.coverage
htmlcov/
.DS_Store
"#.to_string())),

        "gitignore-rust" => Ok((".gitignore", r#"/target/
Cargo.lock
**/*.rs.bk
.env
.DS_Store
*.pdb
"#.to_string())),

        "gitignore-general" => Ok((".gitignore", r#"# OS
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db
desktop.ini

# Editors
.idea/
.vscode/
*.swp
*.swo
*~
.project
.classpath
.settings/
*.sublime-project
*.sublime-workspace

# Secrets
.env
.env.local
*.pem
*.key
secrets.json
credentials.json

# Build artifacts
dist/
build/
out/
"#.to_string())),

        "env-template" => Ok((".env.example", r#"# Application
NODE_ENV=development
PORT=3000
APP_NAME=my-app
LOG_LEVEL=info

# Database
DATABASE_URL=postgres://user:password@localhost:5432/dbname
REDIS_URL=redis://localhost:6379

# Authentication
JWT_SECRET=change-me-in-production
SESSION_SECRET=change-me-in-production

# External APIs
# STRIPE_SECRET_KEY=sk_test_...
# SENDGRID_API_KEY=SG...
# AWS_ACCESS_KEY_ID=AKIA...
# AWS_SECRET_ACCESS_KEY=...
# AWS_REGION=us-east-1

# Feature flags
ENABLE_EXPERIMENTAL=false
"#.to_string())),

        "makefile-node" => Ok(("Makefile", r#".PHONY: install dev build test lint clean docker-build docker-run

install:
	npm ci

dev:
	npm run dev

build:
	npm run build

test:
	npm test

lint:
	npm run lint

clean:
	rm -rf node_modules dist build .next

docker-build:
	docker build -t $(IMAGE_NAME) .

docker-run:
	docker run -p 3000:3000 $(IMAGE_NAME)
"#.to_string())),

        "makefile-python" => Ok(("Makefile", r#".PHONY: install dev test lint format check clean

install:
	pip install -r requirements.txt

dev:
	pip install -r requirements-dev.txt

test:
	pytest

lint:
	ruff check .
	mypy .

format:
	ruff format .

check: lint test

clean:
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -name "*.pyc" -delete
	rm -rf .pytest_cache .mypy_cache .ruff_cache
"#.to_string())),

        "makefile-rust" => Ok(("Makefile", r#".PHONY: build release test check fmt clippy clean doc

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

check:
	cargo check

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

clean:
	cargo clean

doc:
	cargo doc --open
"#.to_string())),

        "docker-compose" => Ok(("docker-compose.yml", format!(r#"version: '3.9'

services:
  web:
    build: .
    ports:
      - "{port}:{port}"
    environment:
      - NODE_ENV=development
      - DATABASE_URL=postgres://app:secret@db:5432/app
      - REDIS_URL=redis://redis:6379
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_started
    volumes:
      - .:/app
      - /app/node_modules

  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: app
    ports:
      - "5432:5432"
    volumes:
      - pg_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U app"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  pg_data:
"#))),

        "pre-commit" => Ok((".pre-commit-config.yaml", r#"repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.6.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-json
      - id: check-toml
      - id: check-merge-conflict
      - id: detect-private-key
      - id: no-commit-to-branch
        args: [--branch, main, --branch, master]

  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.18.4
    hooks:
      - id: gitleaks
"#.to_string())),

        "editorconfig" => Ok((".editorconfig", r#"root = true

[*]
indent_style = space
indent_size = 2
end_of_line = lf
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true

[*.{rs,java,kt}]
indent_size = 4

[*.{md,txt}]
trim_trailing_whitespace = false

[Makefile]
indent_style = tab
"#.to_string())),

        "dependabot" => Ok((".github/dependabot.yml", r#"version: 2
updates:
  - package-ecosystem: "npm"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    open-pull-requests-limit: 5

  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    open-pull-requests-limit: 5

  - package-ecosystem: "pip"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    open-pull-requests-limit: 5

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
"#.to_string())),

        "codeowners" => Ok((".github/CODEOWNERS", format!(r#"# CODEOWNERS — reviewed on every PR
# https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners

*       @{owner}

# Domain ownership
/src/   @{owner}
/docs/  @{owner}
"#, owner=vars.get("owner").map(|s| s.as_str()).unwrap_or("your-team")))),

        "pr-template" => Ok((".github/pull_request_template.md", r#"## Summary

<!-- What does this PR do? Why? -->

## Changes

-
-

## Test plan

- [ ] Unit tests pass
- [ ] Manual test performed
- [ ] Edge cases considered

## Checklist

- [ ] Code follows project conventions
- [ ] No secrets or credentials included
- [ ] Documentation updated if needed
"#.to_string())),

        "issue-bug" => Ok((".github/ISSUE_TEMPLATE/bug_report.md", r#"---
name: Bug report
about: Report a reproducible bug
title: '[BUG] '
labels: bug
---

## Describe the bug

<!-- A clear and concise description of what the bug is. -->

## Steps to reproduce

1.
2.
3.

## Expected behavior

<!-- What did you expect to happen? -->

## Actual behavior

<!-- What actually happened? -->

## Environment

- OS:
- Version:
- Browser/Runtime:

## Logs / Screenshots

<!-- Paste relevant logs or attach screenshots. -->
"#.to_string())),

        "issue-feature" => Ok((".github/ISSUE_TEMPLATE/feature_request.md", r#"---
name: Feature request
about: Suggest a new feature or improvement
title: '[FEATURE] '
labels: enhancement
---

## Problem

<!-- What problem does this feature solve? -->

## Proposed solution

<!-- Describe the feature you'd like. -->

## Alternatives considered

<!-- Other approaches you considered. -->

## Additional context

<!-- Any other context, mockups, or examples. -->
"#.to_string())),

        _ => Err(format!(
            "template_gen: unknown template '{name}'. Run template_gen(template: 'list') to see all available templates."
        )),
    }
}
