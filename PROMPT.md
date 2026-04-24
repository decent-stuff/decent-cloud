Analyze the open GitHub issues for `decent-stuff/decent-cloud` (the canonical task list — `TODO.md` is deprecated).

```bash
gh issue list --repo decent-stuff/decent-cloud --state open --limit 200 \
  --label stripe,decent-agents,launch \
  --json number,title,labels,createdAt,updatedAt,body
```

Identify the highest-impact items that:
1. Deliver user-visible value OR remove a real limitation users hit
2. Require actual design decisions and new code, not just connecting existing pieces
3. Can be completed in a single session, e.g. single week from a multi-week effort
4. Are aligned with the current focus (Stripe hardening / Decent Agents MVP / launch prep — see `GROWTH_PLAN.md` in the outer workspace). Issues labeled `deferred-post-launch` are out of scope; do not pick them.

Pick AS MANY OF THEM AS YOU CAN and complete them with subagents — one subagent per issue.

For each issue, first build a working PoC as per the mandatory workflow. The PoC should be a python or bash script, or a unit/integration test — whichever fits best — and avoid one-off shell commands. After the PoC works, add a failing test for the expected functionality, then get the tests to pass with code changes, all in line with TDD.

When done with an issue:
- Update the GH issue: comment with what shipped (PR link, key decisions, follow-up sub-issues if any), then close it.
  - `gh issue comment <N> --repo decent-stuff/decent-cloud --body "..."`
  - `gh issue close <N> --repo decent-stuff/decent-cloud`
- If new follow-up work surfaces, file new GH issues with the appropriate label (`stripe`, `decent-agents`, or `launch`) and link them from the parent.

Finally each subagent commits the VALUABLE part of its changes (one PR per issue when possible).

If there are not enough MEANINGFUL in-scope issues to do now, create a few subagents for each of the following — but only file findings against the current focus areas:
- Review the codebase for zombie code, inconsistencies, and half-baked things in the Stripe path or the Decent Agents-relevant code (provisioning, dispatcher, billing, webhooks). Fix gaps you can fix; otherwise file a GH issue with the right label.
- Consider the app from a *Decent Agents customer* point of view — onboarding, dashboard, agent-run experience. If a UI/UX change would radically improve intuitiveness for that customer, file an issue with the `launch` label.

Note: do not do UX review and improvements yourself — there is a separate agent for that.

When fully done, use subagents to (a) verify completeness and find implementation gaps, (b) fix small gaps immediately or file GH issues for large ones, and (c) confirm no orphaned in-flight work.

Then do a final pass through `git diff`, analyze leftover changes, and COMMIT all VALUABLE changes, or revert/leave changes that should not be committed, as appropriate. Push when commits are ready.
