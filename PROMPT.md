Analyze `TODO.md`.

Identify the highest-impact items that:
1. Deliver user-visible value OR remove a real limitation users hit
2. Require actual design decisions and new code, not just connecting existing pieces
3. Can be completed in a single session, e.g. single week from a multi-week effort.

Pick AS MANY OF THEM AS YOU CAN and complete them with subagents - one subagent per item.

For each item, first build a working PoC as per the mandatory workflow. The working PoC should be built as python or bash scripts, or unit/integration tests - whichever is more appropriate, and avoid one-off shell commands. After you have a working PoC, add a failing test for the expected item functionality, then get the tests to pass with code changes, all in line with TDD.

Then update `TODO.md` to a) remove all fully done items, b) update existing items with ANY NEW DETAILS, c) splitting into subitems, d) adding dependencies etc. that you may now have on them (IF VALUABLE for future implementation or activities).

If there are not enough MEANINGFUL items in `TODO.md` that you can do now, create a few subagents for each of the following:
- Review the entire codebase and find zombie code and docs, inconsistencies, and half-baked things. Is everything ready for prod? Fix any gaps that are found and if you cannot fix them immediately, add them to `TODO.md`.
- consider the app from the *user point of view* - we have a LOT of functionality that is not visible or not user friendly / easily usable for users, or if there are some UI/UX changes that would RADICALLY improve intuitiveness and usability from the user point of view (simple, clean, obviously usable UI, etc. - less is more!). Fix these found gaps in another subagent if possible, otherwise add them to `TODO.md` and we'll handle them in the follow-up session(s).
Regardless of how radical these changes are - let's do them! And use subagents extensively.

When fully done, use subagents to a) verify completeness and find remaining gaps, and b) fix gaps immediately if small and easy, or add them to `TODO.md` if very large, c) remove all fully done items from `TODO.md` and reorganize `TODO.md` for size and readability.

Then COMMIT all changes, ideally as separate commits.
