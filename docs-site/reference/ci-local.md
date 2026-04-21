# CI on the self-hosted macOS runner

GitHub-hosted runners are unavailable for this repo because the Actions
spending cap is exhausted. All CI for `hwLedger` runs on a self-hosted macOS
runner installed on the maintainer's workstation.

## Why self-hosted

- GitHub Actions billing on the `KooshaPari` account is exhausted; hosted
  runners (`macos-*`, `ubuntu-*`, `windows-*`) fail immediately with a
  billing error and cannot be re-enabled without adding funds.
- Self-hosted runners use $0 of billable minutes.
- Apple Silicon native tests (MLX sidecar, codesigned `.app` bundles) require
  an ARM64 macOS host anyway.

## Workflow: `.github/workflows/ci-local.yml`

Triggers:

- `push` to `main`
- `pull_request` targeting `main`
- `workflow_dispatch` (manual)

Steps:

1. Checkout.
2. Show toolchain versions (`rustc`, `cargo`, `bun`, `uv`).
3. `cargo test --workspace --locked`.
4. `bun install && bun run build` in `docs-site/`.

Job selector: `runs-on: [self-hosted, self-hosted-mac]`.

## Installing the runner

Installed at `~/.github-actions-runner/`. Managed via launchd so it
survives reboots.

### One-time setup

```bash
# 1. Download the latest runner (arm64 macOS).
mkdir -p ~/.github-actions-runner && cd ~/.github-actions-runner
RV=$(curl -s https://api.github.com/repos/actions/runner/releases/latest \
  | grep tag_name | sed -E 's/.*"v([^"]+)".*/\1/')
curl -sSLO "https://github.com/actions/runner/releases/download/v${RV}/actions-runner-osx-arm64-${RV}.tar.gz"
tar xzf "actions-runner-osx-arm64-${RV}.tar.gz"

# 2. Get a short-lived registration token (gh auth scope: repo).
TOKEN=$(gh api -X POST /repos/KooshaPari/hwLedger/actions/runners/registration-token --jq .token)

# 3. Register with label `self-hosted-mac`.
./config.sh \
  --url https://github.com/KooshaPari/hwLedger \
  --token "$TOKEN" \
  --name "$(hostname)-hwledger" \
  --labels "self-hosted,self-hosted-mac,macOS,ARM64" \
  --unattended --replace

# 4. Install + start as launchd service.
./svc.sh install
./svc.sh start
./svc.sh status
```

The launchd plist is installed at
`~/Library/LaunchAgents/actions.runner.KooshaPari-hwLedger.<host>-hwledger.plist`
and auto-loads on login.

### Operational commands

```bash
cd ~/.github-actions-runner

./svc.sh status    # current run state + pid
./svc.sh stop      # stop accepting jobs
./svc.sh start     # resume
./svc.sh uninstall # remove launchd service (runner config stays)

# Completely deregister (use for decommission):
TOKEN=$(gh api -X POST /repos/KooshaPari/hwLedger/actions/runners/remove-token --jq .token)
./config.sh remove --token "$TOKEN"
```

### Logs

- Runner stdout: `~/Library/Logs/actions.runner.KooshaPari-hwLedger.<host>-hwledger/`
- Job workspace: `~/.github-actions-runner/_work/hwLedger/hwLedger/`
- Per-job diagnostics: `~/.github-actions-runner/_diag/`

## Required host toolchain

The runner reuses the user's environment, so these must be on `PATH` at
runner launch time:

| Tool | Install | Purpose |
|------|---------|---------|
| `rustc` / `cargo` | [rustup](https://rustup.rs) | `cargo test --workspace` |
| `bun` | [bun.sh](https://bun.sh) | `docs-site` build |
| `uv` (optional) | [uv](https://docs.astral.sh/uv/) | Streamlit journeys |

## Scopes required on the `gh` CLI

The registration token endpoint
(`POST /repos/:owner/:repo/actions/runners/registration-token`) requires
the `repo` scope — which the account already has. No extra scopes needed
beyond what `gh auth login` provides by default.

If `gh` reports "Resource not accessible by integration" for the
registration-token endpoint, the usual fix is:

```bash
gh auth refresh -h github.com -s admin:org,workflow,repo
```

## Branch protection

CI is advisory only; the user has removed required status checks on `main`
because hosted-runner jobs cannot succeed. The self-hosted `test` job
effectively gates PRs by convention. If it fails:

- Read logs under `~/.github-actions-runner/_work/`.
- Re-run via `gh run rerun <run-id>`.
- Or run locally: `cargo test --workspace && (cd docs-site && bun run build)`.
