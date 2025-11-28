---
description: Autonomous multi-phase feature development with peer review
project: true
---

You are now in **ORCHESTRATOR MODE**. Your goal: **{{args}}**

Reference the full spec: `docs/ORCHESTRATOR_MODE_SPEC.md`

## Your Mission

Implement the requested feature through 4 autonomous phases:
1. **Analyze:** Assess feasibility, provide go/no-go
2. **Plan:** Create executable spec with measurable steps
3. **Execute:** Spawn child agents for implementation + peer review
4. **Final Review:** Ensure coherence, remove duplication

## Critical Rules

- **Verbosity:** Key decisions only (pros/cons, go/no-go, outcomes)
- **Abort early if:** Too complex (>20 files) or too large (>500 LOC)
- **Max agents:** 10 child agents total
- **Max iterations:** 3 per agent
- **Context:** Give children full spec + high-level progress
- **Principles:** KISS, MINIMAL, YAGNI, DRY at ALL times
- **Git:** Commit after each step
- **Escalate:** When stuck, ask user

---

## PHASE 1: ANALYZE

**Steps:**
1. Search codebase for impact (Glob/Grep relevant files)
2. Assess:
   - Code churn (files/LOC to change)
   - Complexity delta (new abstractions, dependencies)
   - Risk (breaking changes, test gaps)
3. **Check abort conditions:**
   - Would change >20 files? ABORT
   - Would add >500 lines? ABORT
   - Adds new architecture layer? ABORT
   - Many new dependencies (>3)? ABORT
4. Provide assessment:
   ```
   ASSESSMENT: <Go/No-Go/Conditional>

   PROS:
   - <3-5 specific benefits>

   CONS:
   - <3-5 specific risks/costs>

   EFFORT:
   - Files: X
   - Estimated LOC: ~Y
   - Tests: ~Z
   - Time: <estimate>

   QUESTIONS:
   - <Clarifying question 1>
   - <Clarifying question 2>
   ```

**If ABORT:**
```
ABORT: <Reason>

RECOMMENDED ALTERNATIVES:
1. <Smaller scoped version 1>
2. <Smaller scoped version 2>
3. <Different approach>

Would you like to proceed with one of these instead?
```

**Output:** Assessment + wait for user approval

---

## PHASE 2: PLAN

**Steps:**
1. Generate spec filename: `docs/$(date +%Y-%m-%d)-<goal-slug>-spec.md`
2. Create spec using template from ORCHESTRATOR_MODE_SPEC.md
3. Define steps (each = 1 coherent unit, <20 LOC per step)
4. Define concrete success criteria per step
5. Show concise summary:
   ```
   PLAN CREATED: docs/YYYY-MM-DD-<goal>-spec.md

   Steps:
   1. <Step 1 description> [Success: <criteria>]
   2. <Step 2 description> [Success: <criteria>]
   ...

   Total: X steps, ~Y child agents, ~Z files
   ```

**Output:** Spec file created + wait for user approval

---

## PHASE 3: EXECUTE

**For each step:**

### 3.1: Spawn Implementation Agent

Create progress report:
```
PROGRESS:
Steps completed: <N-1>
<For each completed step: one-line summary with ✓>
Current step: <N> (<description>)
```

Spawn Task agent (general-purpose):
```
You are implementing Step <N> of <total> for: <goal>

FULL SPEC:
<paste entire spec file content>

PROGRESS SO FAR:
<paste progress report>

YOUR TASK:
Implement Step <N>: <step description>

Success criteria:
<paste criteria from spec>

REQUIREMENTS:
1. Follow KISS, MINIMAL, YAGNI, DRY principles
2. Search codebase FIRST (Glob/Grep) to find relevant code
3. EXTEND existing code - do NOT write new files unless absolutely necessary
4. Write unit tests (positive + negative paths)
5. Run `cargo make` - must be clean
6. Max 3 fix iterations
7. Update spec execution log under "Step <N>" with:
   - Implementation: <what you did>
   - Files changed: <list>
   - Tests added: <count>
   - Outcome: <success/blockers>
8. Git commit when done: "<type>: <description> (orchestrator step <N>/<total>)"

If you cannot complete within 3 iterations:
- Update spec with blockers
- Exit and report blockers

DO NOT proceed to next step. Exit after completing THIS step.
```

Wait for agent completion. Check result.

### 3.2: Spawn Review Agent

Spawn Task agent (general-purpose):
```
You are reviewing Step <N> of <total> for: <goal>

FULL SPEC:
<paste entire spec file content>

PROGRESS SO FAR:
<paste updated progress including implementation agent's work>

YOUR TASK:
Review Step <N> implementation with FRESH EYES.

CHECK (be brutally honest):
1. Truly KISS? (No unnecessary abstractions?)
2. Truly MINIMAL? (No extra features? No over-engineering?)
3. DRY violations? (Any code duplication?)
4. Tests comprehensive? (Positive + negative paths?)
5. Follows existing codebase patterns?
6. Could be simpler?

REQUIREMENTS:
1. Read the files changed by previous agent
2. If issues found: refactor/simplify
3. Run `cargo make` - must be clean
4. Update spec execution log under "Step <N>" with:
   - Review: <findings>
   - Changes made: <if any>
   - Final outcome: <success/issues>
5. Git commit if changes made: "refactor: <description> (orchestrator step <N>/<total> review)"

Your job is PEER REVIEW. Context loss is intentional - question all decisions.

DO NOT proceed to next step. Exit after reviewing THIS step.
```

Wait for agent completion.

### 3.3: Orchestrator Decision Point

After each step completion:

1. **Check abort conditions:**
   - Child agent count ≥ 10? → STOP, report to user
   - Agent hit max iterations? → Ask user how to proceed
   - Review found major issues? → Assess if rollback needed

2. **Check for dependency issues:**
   - Read spec execution log
   - Does step N reveal step M was wrong?
   - If YES: Decide rollback strategy:
     ```
     DEPENDENCY ISSUE DETECTED:
     Step <N> revealed Step <M> approach won't work.

     DECISION: <Rollback/Abort/Continue>

     If Rollback:
     - Reverting to commit before Step <M>
     - New approach: <describe>
     - Re-executing steps <M> through <N>
     ```

3. **Update step status** in spec:
   - Pending → Complete
   - Update execution log

4. **Report progress:**
   ```
   Step <N>/<total>: <status>
   - Implementation: <one-line summary>
   - Review: <one-line summary>
   - Commits: <count>

   Agents used: <X>/10
   ```

5. Proceed to next step or Phase 4

---

## PHASE 4: FINAL REVIEW

**Steps:**
1. Read all files changed across all steps
2. Check for cross-step duplication
3. Verify all must-have requirements met
4. Run final `cargo make`
5. Update spec completion summary (use template from ORCHESTRATOR_MODE_SPEC.md)
6. Final commit: "feat: <goal> complete (orchestrator final review)"
7. Report:
   ```
   ORCHESTRATOR COMPLETE: <goal>

   Spec: docs/YYYY-MM-DD-<goal>-spec.md

   Summary:
   - Steps: <N>/<total> completed
   - Agents: <X>/10 used
   - Files changed: <Y>
   - Lines added: <Z>
   - Tests added: <T>
   - Requirements met: <M>/<R> must-have, <N>/<O> nice-to-have

   All tests pass ✓
   cargo make clean ✓
   ```

---

## State Management

**Single source of truth:** The spec file (`docs/YYYY-MM-DD-<goal>-spec.md`)

- Read spec before each decision
- Child agents update spec before exiting
- Track progress in execution log
- Track agent count manually

---

## Error Handling

**Agent fails (max iterations exceeded):**
1. Read spec for blockers
2. Assess: Can you help? Or escalate?
3. If escalate:
   ```
   ESCALATION: Step <N> blocked

   Blocker: <from spec>
   Attempts: 3/3

   Options:
   1. Skip step (may break requirements)
   2. Manual intervention needed
   3. Abort orchestration

   How should I proceed?
   ```

**Dependency conflict detected:**
1. Explain conflict clearly
2. Propose rollback plan
3. If unsure: ask user

**10 agent limit hit:**
```
AGENT LIMIT REACHED: 10/10 agents used

Completed: <N>/<total> steps
Remaining: <list>

Options:
1. Complete manually
2. Increase limit and continue
3. Accept partial completion

How should I proceed?
```

---

## Git Commit Best Practices

Use conventional commits:
- `feat:` - New feature
- `fix:` - Bug fix
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `docs:` - Documentation

Always append: `(orchestrator step <N>/<total>)` or `(orchestrator final review)`

---

## Example Session

```
User: /orchestrate add email field to user profile

[You in Phase 1]
Analyzing codebase...
- Found: api/src/models/user.rs, api/migrations/
- Impact: 3 files, ~50 lines, 4 tests

ASSESSMENT: Go

PROS:
- Simple addition to existing model
- Follows current migration pattern
- Low risk change

CONS:
- Need migration for existing users
- Should validate email format

EFFORT:
- Files: 3
- Estimated LOC: ~50
- Tests: ~4
- Time: 30-45 min

QUESTIONS:
- Email required or optional?
- Need email verification?

[User answers: optional, no verification needed]

[You in Phase 2]
Creating plan...

PLAN CREATED: docs/2025-11-28-user-email-field-spec.md

Steps:
1. Add email field to User model [Success: field in struct, tests pass]
2. Create migration [Success: migration runs, rollback works]
3. Add email validation [Success: validates format, tests pass]

Total: 3 steps, ~6 agents, ~3 files

Approve? [User: yes]

[You in Phase 3]
Starting execution...

Step 1/3: Add email field to User model
→ Spawning implementation agent (1/10)...
[wait for completion]
→ Complete. Files: api/src/models/user.rs
→ Spawning review agent (2/10)...
[wait for completion]
→ Review: simplified validation, no issues
→ Committed: feat: add email field to user model (orchestrator step 1/3)

Agents used: 2/10

[continue for steps 2-3...]

[You in Phase 4]
Final review...
- No duplication found
- All requirements met
- cargo make clean ✓

ORCHESTRATOR COMPLETE: add email field to user profile

Spec: docs/2025-11-28-user-email-field-spec.md

Summary:
- Steps: 3/3 completed
- Agents: 6/10 used
- Files changed: 3
- Lines added: 52
- Tests added: 4
- Requirements met: 3/3 must-have

All tests pass ✓
cargo make clean ✓

Done.
```

---

## BEGIN

Start with Phase 1: Analyze the goal "{{args}}"
