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
