# Agent Rules

## Deployment

- When the user asks to deploy, do not stop after committing or pushing code.
- Push the relevant branch and deployment tag, then confirm the GitHub Actions
  release workflow has started.
- Monitor the deployment workflow until it reaches a terminal state
  (`success`, `failure`, `cancelled`, or `timed_out`).
- Report the final workflow conclusion, release tag, and any failed job/step
  evidence available from the deployment logs.
- If GitHub CLI is unavailable, use `git` plus the GitHub REST API to trigger
  and poll deployment state.

## Windows Remote Development Loop

- When the user says to build, test, or continue development on Windows, use the
  remote Windows workstation instead of trying to cross-compile the app on macOS.
- The macOS checkout remains the primary editing workspace:
  `/Users/bytedance/rust/AutoAim`.
- The Windows workstation is reachable with `ssh Admin@192.168.1.13`.
- The Windows checkout path is `D:/projects/AutoAim`.
  `D:/projects/AutiAim` was checked on 2026-07-03 and did not exist.
- Treat GitHub as the source of truth between machines. Make the change on macOS,
  commit it, push the branch to GitHub, then pull that branch on Windows before
  building or testing.
- Do not overwrite local changes in the Windows checkout. If `git status --short`
  on Windows is not clean, report it and ask before proceeding.
- The current Windows SSH PowerShell setup prepends a broken `-l` token to remote
  commands. Prefix remote PowerShell command strings with `;` so the intended
  command still runs.

Typical flow from macOS:

```bash
branch=$(git branch --show-current)
git push origin "$branch"

ssh Admin@192.168.1.13 "; Set-Location 'D:\projects\AutoAim'; git status --short; git fetch origin; git checkout $branch; git pull --ff-only origin $branch"

ssh Admin@192.168.1.13 '; Set-Location "D:\projects\AutoAim"; New-Item -ItemType Directory -Force ".e2e-output" | Out-Null; cargo test -p autoaim-core -p autoaim-ipc -p autoaim-runtime -p autoaim-capture -p autoaim-infer -p autoaim-cli *> ".e2e-output\cargo-test.log"; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'

ssh Admin@192.168.1.13 '; Set-Location "D:\projects\AutoAim"; cargo build -p autoaim-cli *> ".e2e-output\cargo-build-cli.log"; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; python tests\rust_cli_e2e.py *> ".e2e-output\rust-cli-e2e.log"; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'

ssh Admin@192.168.1.13 '; Set-Location "D:\projects\AutoAim"; python scripts\prepare_models.py *> ".e2e-output\prepare-models.log"; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; python scripts\build_windows_package.py --output-dir dist\windows *> ".e2e-output\windows-package.log"; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'
```

After a failure, inspect logs on Windows and copy the relevant evidence back into
the macOS-side investigation:

```bash
ssh Admin@192.168.1.13 '; Get-ChildItem "D:\projects\AutoAim\.e2e-output\*.log" | Sort-Object LastWriteTime | Select-Object FullName,Length,LastWriteTime'
ssh Admin@192.168.1.13 '; Get-Content "D:\projects\AutoAim\.e2e-output\cargo-test.log" -Tail 200'

mkdir -p .remote-logs/windows
scp 'Admin@192.168.1.13:D:/projects/AutoAim/.e2e-output/*.log' .remote-logs/windows/
```

Use the Windows logs and generated package output under `dist/windows` to guide
the next macOS edit, then repeat the push/pull/build/test loop.
