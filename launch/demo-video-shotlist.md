# Demo video shotlist -- Decent Agents (60--90s)

**Status:** draft for founder review. Cannot be recorded until #418 (cold-onboarding flow) is functional. Re-shoot if any UI changes between draft and recording.

**Format:** 1080p screen recording, OBS or QuickTime, 30fps. No webcam overlay. Voiceover recorded separately and synced.

**Voice:** founder's own. No music. No motion graphics. The product is the demo.

**Total length target:** 75--85 seconds. Hard cap 90s (above this, drop-off rate doubles per a16z viewer-retention data).

**Pacing rule:** show, do not tell. Voiceover narrates what is happening on screen, never describes a feature in marketing terms.

---

## Shot 1 -- Cold-open hook (0:00--0:08)

- **Screen:** GitHub Issues tab of a real-looking repo. ~40 open issues visible, a mix of "bug:", "chore:", "feat:" labels. Cursor scrolls slowly through them.
- **On-screen narration text (subtitle, optional):** none. Let the screen speak.
- **Voiceover:** "If your GitHub backlog looks like this, the next 60 seconds are for you."

## Shot 2 -- One-line pitch over landing page (0:08--0:18)

- **Screen:** `agents.decent-cloud.org` hero. Cursor lands on the hero headline. Pricing block visible: "CHF 49 / month. 1 agent. 20 hours. EU-hosted."
- **On-screen narration text:** none -- the page already has the words.
- **Voiceover:** "Decent Agents. A dedicated AI engineer for your GitHub repo. Forty-nine francs a month. EU-hosted. Cancel any time."

## Shot 3 -- Sign up + Stripe checkout (0:18--0:30)

- **Screen:** Click "Get started". Email + password signup form fills (use a saved-credential autofill so it is two seconds, not twenty). Click "Subscribe -- CHF 49/month". Stripe Checkout slides in. Test card autofills, click Pay.
- **Voiceover:** "Sign up. Pay. That is the gate."
- **Note:** if Stripe Checkout takes >5s in production, cut to "Subscription active" confirmation screen and trim. Do not film waiting.

## Shot 4 -- Install GitHub App (0:30--0:45)

- **Screen:** "Your agent is ready" page in dashboard. CTA: "Install GitHub App". Click. GitHub OAuth screen. Pick one repo (NOT all repos) -- show the radio button selection clearly. Click Install.
- **On-screen narration text (overlay):** "Single repo, scoped permissions"
- **Voiceover:** "Install the GitHub App on one repo. The agent only sees this repo. The token only works here."

## Shot 5 -- File a real issue (0:45--0:55)

- **Screen:** GitHub Issues -> New Issue. Title: "Add timezone support to scheduled reports". Body (3--4 lines): "Currently `scheduled_at` is interpreted as UTC. Users in CET expect local time. Update the scheduler to accept a `tz` field and convert on dispatch." End with `@decent-agent please take a look`.
- **Voiceover:** "File an issue. Mention the agent. That is the API."
- **Note:** the issue must be a real-shaped issue -- specific, scoped, with clear acceptance. If it is vague, the agent's PR will be vague and the demo collapses.

## Shot 6 -- Time-lapse to PR opened (0:55--1:08)

- **Screen:** Wipe transition or fade to a 4x time-lapse of the dashboard "Activity" feed. Show: "agent run started", "branch created", "tests running", "PR opened". End on the GitHub PR page itself with a real diff visible (tz field added, scheduler test updated). Diff is small (~30 lines), readable in 3 seconds.
- **On-screen narration text (overlay):** "~6 minutes elapsed"
- **Voiceover:** "Six minutes later there is a pull request. With a diff. With a passing test."

## Shot 7 -- Review the PR (1:08--1:18)

- **Screen:** Hover the diff. Cursor moves over the timezone conversion logic. Click "Files changed". Click "Approve". Click "Squash and merge". Confirm.
- **Voiceover:** "Read it like any PR. Reject it, request changes, or merge it. The agent does not push to main. You do."

## Shot 8 -- Close + CTA (1:18--1:25)

- **Screen:** GitHub repo Issues tab. The original issue now shows "Closed by PR #N". Pan to the dashboard usage page: "0.4 hours used / 20. 12k tokens used / 3M."
- **On-screen narration text (overlay, last 2s):** `agents.decent-cloud.org | CHF 49 / month`
- **Voiceover:** "One issue closed. Nineteen point six hours left this month. Try it on your repo. Link below."

---

## Production checklist

- [ ] Repo used in demo is a fresh demo-repo with realistic but non-confidential code. Do NOT film against a real customer repo or a real internal repo.
- [ ] Stripe is in test mode. The card number visible must be `4242 4242 4242 4242`.
- [ ] GitHub demo account is `decent-agents-demo`, not the founder's personal account.
- [ ] All UI shown matches production at recording time. If UI changes within 14 days of recording, reshoot.
- [ ] Audio: dry voiceover, normalised to -14 LUFS. No music bed.
- [ ] Subtitles burned in (not closed-caption -- platforms strip them). Sans-serif, white text on dark drop-shadow, lower-third placement.
- [ ] Final export: H.264, 1080p, ~10MB target. Hosted on the landing page directly (not YouTube embed) to keep load fast and remove YouTube's "up next" surface.
- [ ] Backup low-res (480p) version for HN thread (HN viewers click links from mobile in transit).

## What NOT to do

- No "imagine if..." opening. Cut.
- No founder face on camera. Adds nothing, costs trust if framing is off.
- No fake speed-up of the agent run. The 4x time-lapse is honest because the timer is shown.
- No comparison-to-competitor screen. HN audience hates it; SaaS buyers can compare themselves.
- No "and many more features" tease. Show what exists. Do not promise.
