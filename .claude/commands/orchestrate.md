---
description: Autonomous multi-phase feature development with peer review
project: true
---

You are in **ORCHESTRATOR MODE**. Goal: **{{args}}**

Execute through 4 phases: Analyze → Plan → Execute → Final Review

**Limits:** Max 15 agents (≤3 per step), 3 iterations/agent, 200 tool calls/agent
**Principles:** KISS, MINIMAL, YAGNI, DRY. Report key decisions only.

---

## PHASE 1: ANALYZE

Search codebase, assess: code churn (files/LOC), complexity delta, risk (breaking changes, test gaps)

**Abort if:** >30 files, >10000 LOC, or >5 dependencies

**Output:**
```
ASSESSMENT: Go/No-Go
PROS: <3-5 benefits>
CONS: <3-5 risks>
EFFORT: Files: X, LOC: ~Y, Tests: ~Z, Time: <est>
QUESTIONS: <if needed>
```

If abort: suggest 2-3 smaller alternatives. Wait for user approval before Phase 2.

---

## PHASE 2: PLAN

Create `docs/YYYY-MM-DD-<goal>-spec.md`:

```markdown
# <Goal>
**Status:** In Progress

## Requirements
### Must-have
- [ ] Requirement 1

### Nice-to-have
- [ ] Optional 1

## Steps
### Step N: <Description>
**Success:** <specific criteria>
**Status:** Pending

## Execution Log
### Step N
- **Implementation:** <summary>
- **Review:** <summary>
- **Verification:** <evidence (E2E/API/frontend/migrations only)>
- **Outcome:** <success/failure>

## Completion Summary
<Filled in Phase 4>
```

**Report:**
```
PLAN: docs/YYYY-MM-DD-<goal>-spec.md
Steps: 1. <desc> 2. <desc> ...
Total: X steps, ~Y agents
```

Wait for user approval before Phase 3.

---

## PHASE 3: EXECUTE

**Common to all agents:**
- Paste SPEC + PROGRESS at start
- Update spec execution log before exit
- Exit after assigned step only
- **Anti-loop:** State expected output before commands. If no output or same output 2x: change approach. If stuck after 2 attempts: update spec with blockers, EXIT. 200 tool call limit.

### 3.1: Implementation

Task prompt:
```
Step <N>/<total> for: <goal>
[SPEC + PROGRESS]

TASK: Implement Step <N>

REQUIREMENTS:
1. KISS, MINIMAL, YAGNI, DRY
2. Search codebase FIRST, extend existing code
3. Write tests (positive + negative)
4. Run cargo make (must be clean)
5. Update spec execution log: implementation, files, tests, outcome
6. Git commit: "<type>: <desc> (orchestrator step <N>/<total>)"
7. Max 3 fix iterations → EXIT with blockers
```

### 3.2: Review (Code Quality)

Task prompt:
```
Review Step <N>/<total> for: <goal>
[SPEC + PROGRESS]

CHECK: KISS/MINIMAL? DRY? Tests comprehensive? Codebase patterns? Simpler?

REQUIREMENTS:
1. Read changed files
2. Refactor/simplify if needed
3. Run cargo make
4. Update spec: findings, changes, outcome
5. Git commit if changed: "refactor: <desc> (orchestrator step <N>/<total> review)"
```

### 3.3: Verification (Independent Validation)

**MANDATORY for:** E2E tests, API endpoints, frontend components, integration tests, database migrations

Task prompt:
```
Verify Step <N>/<total> for: <goal>
[SPEC + PROGRESS]

RULES: Zero trust. Execute tests/features yourself. Report OBJECTIVE evidence.

VERIFICATION (pick relevant):
- E2E: Run npx playwright test <file>. Verify passes, selectors match DOM
- API: Start server, send HTTP requests, verify responses + errors
- Frontend: Run npm run dev, interact in browser, test edge cases
- Migrations: Fresh DB, run migrations, verify schema

REQUIREMENTS:
1. Execute actual tests/servers (not just static analysis)
2. Document objective evidence
3. If fails: update spec with BLOCKER, exit
4. If passes: update spec with verification evidence
5. Git commit: "test: verify <desc> (orchestrator step <N>/<total> verification)"
```

### 3.4: Decision Point

**Loop detection:** Spec not updated? Same error? Same files 3+ times? → ESCALATE

**Abort:** 15 agents used | Max iterations | Loop → STOP/ESCALATE

**Dependency issue:** Step N reveals step M wrong? → Rollback to commit before M or abort

**Validation:**
- Implementation: files changed + ≥1 commit
- Review: findings + (no issues OR commits)
- Verification (if applicable): objective evidence (test output, HTTP responses, server logs, migration output)
- Missing/failed verification: ESCALATE with blocker

**Report:** `Step <N>/<total>: <status> | Impl: <summary> | Review: <summary> | Verify: <evidence> | Agents: <X>/15`

---

## PHASE 4: FINAL REVIEW

1. Read all changed files
2. Remove cross-step duplication
3. Verify requirements met
4. Run cargo make
5. Update spec:
   ```markdown
   ## Completion Summary
   **Completed:** YYYY-MM-DD | **Agents:** X/15 | **Steps:** Y/Z
   Changes: X files, +Y/-Z lines, N tests
   Requirements: M/R must-have, N/O nice-to-have
   Tests pass ✓, cargo make clean ✓
   Notes: <key decisions, trade-offs>
   ```
6. Commit: "feat: <goal> complete (orchestrator final review)"
7. Report:
   ```
   COMPLETE: <goal>
   Spec: docs/YYYY-MM-DD-<goal>-spec.md
   Steps: Y/Z | Agents: X/15 | Files: F | Tests: T
   All requirements met ✓
   Verification: <summary of verified components>
   ```

---

## BEGIN

Start Phase 1: Analyze "{{args}}"
