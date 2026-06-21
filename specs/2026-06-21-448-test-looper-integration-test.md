# Spec: Looper Integration Test for jcode

> **Issue**: [#448](https://github.com/quangdang46/jcode/issues/448) – TEST: Looper integration test for jcode
> **Date**: 2026-06-21
> **Status**: Draft / Proposed

---

## Problem

Looper is an autonomous AI dev team for GitHub repos that can detect issues, plan fixes, implement them, and open PRs. However, there is currently no automated integration test that validates the full Looper pipeline against the jcode repository:

1. Picking up a `looper:plan`-labeled issue
2. Running the planner agent to produce a spec
3. Reviewing the spec via the reviewer agent
4. Implementing the fix via fixer/worker agents
5. Opening a spec PR and driving it to completion

Without this test, regressions in Looper's GitHub integration, agent orchestration, or PR lifecycle management can go undetected, making upstream Looper changes risky to deploy.

---

## Goals

1. **Create a repeatable integration test** that exercises the full Looper pipeline from issue detection → spec PR → review → implementation.
2. **Verify automated PR creation** — confirm Looper opens a spec PR on the correct branch with the correct base.
3. **Validate agent-driven spec writing** — the planner agent produces a structured markdown spec covering problem, goals, approach, risks, and validation.
4. **Validate agent loop handoffs** — planner output feeds reviewer/fixer/worker without manual intervention.
5. **Produce a spec artifact** that the fixer/worker agents can subsequently implement.

### Non-Goals

- End-to-end E2E tests outside of GitHub (this test targets the real Looper ↔ GitHub integration).
- Performance or stress testing of Looper's agent loop.
- Coverage of Looper failure modes (those belong in Looper's own test suite).
- Production-level fix implementation — the fix phase is secondary to validating the pipeline.

---

## Approach

### Phase 1: Spec Creation (this PR)

1. **Issue #448** is labeled `looper:plan` and assigned to `quangdang46`, which triggers the Looper planner agent.
2. The planner agent analyses the issue body, checks the repo context (`AGENTS.md`, codebase structure), and creates this spec document at `specs/2026-06-21-448-test-looper-integration-test.md`.
3. The planner commits the spec on branch `looper/planner/448-test-looper-integration-test` and opens a PR.

### Phase 2: Review

4. The Looper reviewer agent inspects the spec PR for:
   - Completeness (are all sections present?)
   - Feasibility (can the fixer implement this?)
   - Alignment (does the approach match the issue?)
5. Reviewer leaves comments or approves the spec.

### Phase 3: Implementation

6. The Looper fixer/worker agents implement the integration test based on the approved spec.
7. The PR is updated with the implementation and driven to merge.

### Phase 4: Cleanup

8. The test issue (#448) is closed and the `looper:plan` label is removed.
9. The integration test script/configuration is committed to the repo for future Looper QA runs.

---

## Spec Document Structure

Each planning spec in this repository follows this template:

```markdown
# Spec: <Title>

> **Issue**: #NNN – <Issue Title>
> **Date**: YYYY-MM-DD
> **Status**: Draft / Proposed / Approved / Implemented

---

## Problem

<What problem does this solve? Why is it important?>

---

## Goals

<Measurable outcomes. Bullet-list format. Non-goals section if applicable.>

---

## Approach

<High-level technical approach. Phases or steps if the work is large.>

---

## Risks

<What could go wrong? Mitigations.>

---

## Validation

<How do we know it works? Acceptance criteria.>
```

---

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Looper cannot detect `looper:plan`-labeled issues | Pipeline never starts | Ensure label is correctly applied; verify Looper's webhook/subscription config for the repo |
| Spec PR branch conflicts with existing work | Push failure | Use a unique branch name (`looper/planner/448-*`) and rebase before push |
| Reviewer/fixer agents fail to pick up after planner | Pipeline halts mid-way | Looper should handle agent handoffs via its internal state machine; verify the run state persists across agent boundaries |
| Spec is too vague for fixer to implement | Blocked downstream | The spec must include concrete acceptance criteria and file paths |
| Multiple Looper agents writing to the same worktree | Git state corruption | Each agent should use its own worktree; the repo already supports worktree-based agent isolation |
| Looper-generated content disclosure policies violated | Compliance issue | Ensure generated content footers follow the `<!-- looper:stamp v=1 -->` pattern per the policy |

---

## Validation

### Acceptance Criteria

1. **Issue detection:** Looper picks up `#448` based on the `looper:plan` label and `quangdang46` assignment.
2. **Branch creation:** A branch `looper/planner/448-test-looper-integration-test` is created from `master`.
3. **Spec artifact:** `specs/2026-06-21-448-test-looper-integration-test.md` exists in the repo with all required sections.
4. **Spec content:** The spec includes Problem, Goals, Approach, Risks, and Validation sections (this document).
5. **PR created:** A pull request is opened against `master` with the spec changes.
6. **Review completed:** The reviewer agent inspects the PR and either approves or leaves actionable comments.
7. **PR is mergeable:** The PR passes CI checks and is in a mergeable state.

### Manual Verification Steps

```bash
# 1. Confirm the branch exists
git branch --list 'looper/planner/448-*'

# 2. Confirm the spec file exists
ls -la specs/2026-06-21-448-test-looper-integration-test.md

# 3. Confirm PR exists on GitHub
gh pr list --head looper/planner/448-test-looper-integration-test

# 4. Confirm issue is still open with looper:plan label
gh issue view 448 --repo quangdang46/jcode
```
