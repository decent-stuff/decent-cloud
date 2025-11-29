---
description: Autonomous multi-phase feature development with peer review
project: true
---

You are in **ORCHESTRATOR MODE**. Goal: **{{args}}**

Execute through 4 phases: Analyze → Plan → Execute → Final Review

**Limits:** Max 10 child agents, 3 iterations per agent, 50 tool calls per agent (enforced by hook)

**Principles:** KISS, MINIMAL, YAGNI, DRY. Verbosity: key decisions only.

---

## PHASE 1: ANALYZE

Search codebase (Glob/Grep), assess impact:
- Code churn (files/LOC)
- Complexity delta (new abstractions, dependencies)
- Risk (breaking changes, test gaps)

**Abort if:** >20 files, >500 LOC, new architecture layer, or >3 dependencies

**Output format:**
```
ASSESSMENT: Go/No-Go

PROS:
- <3-5 benefits>

CONS:
- <3-5 risks>

EFFORT:
- Files: X, LOC: ~Y, Tests: ~Z, Time: <est>

QUESTIONS: <clarifying questions if needed>
```

**If abort:** Suggest 2-3 smaller alternatives, ask user to choose.

Wait for user approval before Phase 2.

---

## PHASE 2: PLAN

Create `docs/YYYY-MM-DD-<goal-slug>-spec.md`:

```markdown
# <Goal>
**Status:** In Progress

## Requirements
### Must-have
- [ ] Requirement 1
- [ ] Requirement 2

### Nice-to-have
- [ ] Optional 1

## Steps
### Step 1: <Description>
**Success:** Tests pass, cargo make clean, <specific requirement>
**Status:** Pending

### Step 2: ...

## Execution Log
### Step 1
- **Implementation:** <summary>
- **Review:** <summary>
- **Outcome:** <success/failure>

## Completion Summary
<Filled in Phase 4>
```

**Show concise summary:**
```
PLAN: docs/YYYY-MM-DD-<goal>-spec.md
Steps: 1. <desc> 2. <desc> ...
Total: X steps, ~Y agents
```

Wait for user approval before Phase 3.

---

## PHASE 3: EXECUTE

For each step:

### 3.1: Implementation Agent

Spawn Task (general-purpose):
```
Step <N>/<total> for: <goal>

SPEC:
<paste full spec>

PROGRESS:
Steps completed: <N-1>
<one-line summaries with ✓>

TASK: Implement Step <N>: <description>

Success: <criteria from spec>

REQUIREMENTS:
1. KISS, MINIMAL, YAGNI, DRY
2. Search codebase FIRST, extend existing code
3. Write tests (positive + negative)
4. Run cargo make (must be clean)
5. Update spec "Step <N>" execution log with: implementation, files changed, tests added, outcome
6. Git commit: "<type>: <desc> (orchestrator step <N>/<total>)"
7. Max 3 fix iterations, then update spec with blockers and EXIT

ANTI-LOOP:
- State expected output before bash commands
- If no output or same output twice: change approach
- If stuck after 2 attempts: update spec with blockers, EXIT
- Check spec log for failed approaches, try different method
- Hook limits you to 50 tool calls

Exit after THIS step only. Do NOT proceed to next step.
```

### 3.2: Review Agent

Spawn Task (general-purpose):
```
Review Step <N>/<total> for: <goal>

SPEC:
<paste full spec with implementation results>

PROGRESS:
<updated with implementation outcome>

TASK: Review Step <N> with FRESH EYES

CHECK:
1. Truly KISS/MINIMAL? No over-engineering?
2. DRY violations?
3. Tests comprehensive?
4. Follows codebase patterns?
5. Could be simpler?

REQUIREMENTS:
1. Read files changed by previous agent
2. If issues: refactor/simplify
3. Run cargo make
4. Update spec "Step <N>" log: review findings, changes made, final outcome
5. Git commit if changed: "refactor: <desc> (orchestrator step <N>/<total> review)"

ANTI-LOOP:
- State expected output before bash commands
- If same issues as previous review: you're looping, escalate
- Hook limits you to 50 tool calls

Peer review = question all decisions. Exit after THIS step only.
```

### 3.3: Decision Point

After each step, check spec execution log:

**Loop detection:**
- Spec not updated? Same error? Same files 3+ times? → ESCALATE: "Step <N> stuck, spec shows: <evidence>. Options: 1) Manual, 2) Skip, 3) Abort. Proceed?"

**Abort conditions:**
- 10 agents used? → STOP, report
- Max iterations hit? → ESCALATE
- Loop detected? → ESCALATE

**Dependency issues:**
- Step N reveals step M wrong? → Decide: rollback to commit before step M with new approach, or abort

**Progress validation:**
- Implementation: spec must show files changed + ≥1 commit
- Review: spec must show findings + (no issues OR commits)
- Else: escalate

**Update spec:** Mark step complete, report:
```
Step <N>/<total>: <status>
- Implementation: <summary>
- Review: <summary>
- Agents: <X>/10
```

Proceed to next step or Phase 4.

---

## PHASE 4: FINAL REVIEW

1. Read all changed files
2. Remove cross-step duplication
3. Verify requirements met
4. Run cargo make
5. Update spec completion summary:
   ```markdown
   ## Completion Summary
   **Completed:** YYYY-MM-DD
   **Agents:** X/10, **Steps:** Y/Z

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
   Steps: Y/Z, Agents: X/10, Files: F, Tests: T
   All requirements met ✓
   ```

---

## BEGIN

Start Phase 1: Analyze "{{args}}"
