# Orchestrator Mode - Technical Specification

## Overview

Orchestrator mode enables autonomous, multi-phase feature development with peer review and quality gates. It operates within Claude Code using the Task agent system.

## Trigger

```bash
/orchestrate <goal>
```

Example: `/orchestrate add email-based login support`

## Phases

### Phase 1: Analyze (Orchestrator)

**Objective:** Determine feasibility and provide objective go/no-go recommendation.

**Actions:**
1. Examine codebase for impact (Glob/Grep for relevant files)
2. Assess:
   - **Code churn:** Estimated files/lines to change
   - **Complexity delta:** New abstractions, dependencies, architectural changes
   - **Risk:** Breaking changes, test coverage gaps, integration points
3. **Early abort if:**
   - Too much work for single feature (>20 files, >500 lines)
   - Adds excessive complexity (new architecture layer, many new dependencies)
4. Provide concise assessment:
   - **Pros:** Benefits to codebase/users
   - **Cons:** Risks, complexity, maintenance burden
   - **Effort:** Files to change, tests to write, estimated LOC
5. Ask clarifying questions (authentication method? existing vs new patterns?)

**Output:** Concise summary + user approval gate

**Abort example:**
```
ABORT: Too complex for orchestrator mode
- Would require: 35 files changed, new auth layer, 3 new dependencies
- Recommendation: Break into smaller features:
  1. Add email storage to user model
  2. Add email validation service
  3. Integrate email login flow
```

---

### Phase 2: Plan (Orchestrator)

**Objective:** Create executable, measurable plan.

**Actions:**
1. Create `docs/YYYY-MM-DD-<goal-slug>-spec.md`
2. Define steps (each = 1 coherent unit, max 20 lines of code)
3. Define success criteria (concrete, measurable)

**Spec Template:**
```markdown
# <Goal>

**Date:** YYYY-MM-DD
**Status:** Planning | In Progress | Complete | Aborted

## Goal
<One-line description>

## Requirements

### Must-have
- [ ] Specific, testable requirement 1
- [ ] Specific, testable requirement 2

### Nice-to-have
- [ ] Optional enhancement 1

## Steps

### Step 1: <Description>
**Success Criteria:**
- Tests pass
- cargo make clean
- <Specific functional requirement met>

**Status:** Pending | In Progress | Complete | Failed

### Step 2: ...

## Execution Log

### Step 1
- **Implementation:** <Agent summary>
- **Review:** <Agent summary>
- **Outcome:** <Success/failure + key changes>

### Step 2
...

## Completion Summary
<Filled after Phase 4>
```

**Output:** Spec file + user approval gate

---

### Phase 3: Execute (Per Step)

**Objective:** Implement step with peer review.

**Limits:**
- Max 10 child agents total across all steps
- Max 3 fix iterations per agent
- Each step gets 2 agents: Implementation + Review

#### 3.1 Implementation Agent (Child #N)

**Prompt Template:**
```
You are implementing Step X of Y for: <goal>

FULL SPEC:
<entire spec content>

PROGRESS SO FAR:
<execution log entries for completed steps>

YOUR TASK:
Implement Step X: <step description>

Success criteria:
- <criteria from spec>

REQUIREMENTS:
- KISS, MINIMAL, YAGNI, DRY principles
- Write unit tests (positive + negative paths)
- Run tests + cargo make
- Max 3 fix iterations
- Update spec execution log with your work
- Git commit when done

If you cannot complete within 3 iterations, update spec with blockers and exit.
```

**Actions:**
1. Implement minimal code
2. Write tests
3. Run `cargo make`
4. Fix issues (max 3 iterations)
5. Update spec execution log
6. Git commit: `<type>: <description> (orchestrator step X/Y)`

**Escalation:** If 3 iterations exceeded, update spec and exit

#### 3.2 Review Agent (Child #N+1)

**Prompt Template:**
```
You are reviewing Step X of Y for: <goal>

FULL SPEC:
<entire spec content>

PROGRESS SO FAR:
<execution log with implementation agent's work>

YOUR TASK:
Review Step X implementation with fresh eyes.

CHECK:
- Truly KISS? (No unnecessary abstractions?)
- Truly MINIMAL? (No extra features?)
- DRY violations? (Any duplication?)
- Tests cover positive + negative paths?
- Follows codebase patterns?

REQUIREMENTS:
- If issues found: refactor, run cargo make
- Update spec execution log with review findings
- Git commit if changes made
- Be brutally honest

Your job is peer review. Context loss is intentional - question decisions.
```

**Actions:**
1. Review code with fresh context
2. Check KISS/MINIMAL/YAGNI/DRY alignment
3. Refactor if needed
4. Run `cargo make`
5. Update spec execution log
6. Git commit if changes made

#### 3.3 Orchestrator Decision Point (After Each Step)

**Check:**
- Review found major issues + max retries hit? → Escalate to user
- Step N reveals step M was wrong? → Decide: abort or rollback
- 10 child agents spawned? → Stop, report to user
- Step failed? → Ask user how to proceed

**Rollback example:**
```
Step 3 revealed Step 1's database schema is incompatible with Step 2's API.

DECISION: Rollback to commit before Step 1, retry with different approach:
- Instead of: new auth table
- Try: extend users table with email field
```

---

### Phase 4: Final Review (Orchestrator)

**Objective:** Ensure overall coherence and quality.

**Actions:**
1. Check for cross-step duplication
2. Verify all requirements met
3. Run final `cargo make`
4. Update spec completion summary
5. Final commit

**Completion Summary Template:**
```markdown
## Completion Summary

**Completed:** YYYY-MM-DD
**Total child agents:** X/10
**Steps completed:** Y/Z

### What Changed
- Files modified: X
- Lines added: Y
- Lines removed: Z
- Tests added: N

### Quality Metrics
- All tests pass: ✓
- cargo make clean: ✓
- Requirements met: X/Y must-have, Z/W nice-to-have

### Notes
<Key decisions, trade-offs, future work>
```

---

## Abort Conditions

1. **Phase 1:** Too complex or too large
2. **Phase 3:** 10 child agents spawned
3. **Phase 3:** Step exceeds 3 iterations → escalate
4. **Phase 3:** Orchestrator detects dependency conflict → decide

---

## Git Commit Strategy

**Implementation commits:**
```
feat: implement email validation (orchestrator step 1/5)
test: add email validation tests (orchestrator step 1/5)
refactor: simplify email validation (orchestrator step 1/5 review)
```

**Orchestrator commits:**
```
docs: add orchestrator spec for email login
docs: update orchestrator spec - step 1 complete
feat: email login complete (orchestrator final review)
```

---

## Child Agent Context

Each child agent receives:
1. **Full spec** (must be concise!)
2. **Super high-level progress report:**
   ```
   Steps completed: 1-2
   Step 1: Added email field to User model ✓
   Step 2: Added email validation service ✓
   Current step: 3 (integrate login flow)
   ```

---

## Verbosity Level

**Show:**
- ✓ Go/no-go decision with reasoning
- ✓ Pros/cons summary (3-5 bullets each)
- ✓ Step outcomes (success/failure + key changes)
- ✓ Abort reasons
- ✓ Escalation reasons

**Hide:**
- ✗ Code snippets in orchestrator output (in spec only)
- ✗ Detailed file-by-file analysis
- ✗ Individual test results (unless failure)

---

## Example Flow

```
User: /orchestrate add email-based login

[Phase 1: Analyze]
Orchestrator: Analyzing codebase...
- Found: existing password auth in api/src/auth.rs
- Impact: 5 files, ~150 lines, 8 tests
- Risk: Medium (auth changes, migration needed)

PROS:
- Uses existing auth infrastructure
- Standard pattern (email + password)
- Better UX than username-only

CONS:
- Requires email validation
- Need migration for existing users
- Email uniqueness constraint

EFFORT: 5 files, ~150 LOC, 8 tests, 2-3 hours

Questions:
1. Email required or optional for existing users?
2. Send verification email or just validate format?

[User answers]

[Phase 2: Plan]
Orchestrator: Creating spec...
→ docs/2025-11-28-email-login-spec.md

Steps:
1. Add email field to User model + migration
2. Add email validation service
3. Update login endpoint to accept email
4. Add tests for email login flow
5. Update frontend login form

Approve plan? [User: yes]

[Phase 3: Execute]
Orchestrator: Starting execution...

Step 1/5: Add email field to User model + migration
→ Spawning implementation agent (1/10)...
→ Implementation complete (api/src/models/user.rs:45)
→ Spawning review agent (2/10)...
→ Review: simplified migration, added index
→ Committed: feat: add email field to user model (orchestrator step 1/5)

Step 2/5: Add email validation service
→ Spawning implementation agent (3/10)...
→ Implementation complete (api/src/services/email.rs:1)
→ Spawning review agent (4/10)...
→ Review: no issues, tests comprehensive
→ Committed: feat: add email validation service (orchestrator step 2/5)

[...steps 3-5...]

[Phase 4: Final Review]
Orchestrator: Final review...
- Removed duplicate validation in step 3
- All requirements met
- cargo make clean ✓

Completed: docs/2025-11-28-email-login-spec.md
- 5 files changed
- 152 lines added
- 18 tests added
- 8 child agents used

Done.
```

---

## Implementation Notes

- Implemented as `.claude/commands/orchestrate.md`
- Uses Task tool with `subagent_type=general-purpose`
- Orchestrator maintains state in spec file (single source of truth)
- Each child agent updates spec before exiting
- Orchestrator reads spec to track progress
