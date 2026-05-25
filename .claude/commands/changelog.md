Generate a Discord-ready "What's New" changelog from recent git history.

**Input:** `$ARGUMENTS` — either a date (e.g. "May 7", "2026-05-07", "May 7 1pm MST"), a commit ref (e.g. "abc1234", "v0.1.2"), or empty. If empty, default to the last 7 days. For sequential ("next batch") requests, the start boundary is the **tip hash of the previous changelog** (see Step 5).

**Step 0: Sync first.** Run `git fetch origin` so the range is computed against current `origin/main`, not a stale local ref. Use `origin/main` (or whichever branch the user names) as the tip in all `git log` invocations below.

**Step 1: Determine the range — and pick the right tool for it.**

⚠️ **`git log --since` is unreliable on this repo. Do not trust it for the range.** This repo is squash-merge heavy, so commit dates are non-monotonic (a squashed commit's date can predate commits already on the branch). `git --since` prunes the revision walk by date, so it silently drops commits whose date is older than a later commit it already passed — it returned **50 commits when the true count was 101** in one real case. The set it returns even changes with `--date-order` vs default order. Never report a `--since` count as authoritative.

Use the method that matches the request:

- **Commit hash / tag start (preferred — and required for sequential batches):** use a ref-range `<ref>..origin/main`. This is graph-reachability based, immune to date pruning. This is the reliable path — anchor to a hash whenever you can.
  ```bash
  git log --no-merges <ref>..origin/main --format="%h %s" | cat -n
  ```

- **Date/time start:** convert the input **yourself** to a Unix epoch, then filter on `%ct` (committer date, epoch) with `awk` — this is offset-agnostic and does not depend on the graph walk, so it recovers every commit `--since` would drop:
  ```bash
  # Convert the input to an epoch cutoff (macOS/BSD date):
  cutoff=$(date -j -f "%Y-%m-%dT%H:%M:%S %z" "2026-05-22T21:05:00 -0700" "+%s")
  # Offset-agnostic, graph-walk-independent filter:
  git log origin/main --no-merges --format="%ct %cI %h %s" \
    | awk -v c="$cutoff" '$1 >= c' | sort -rn | cat -n
  ```
  Convert named timezones yourself before computing the epoch — never hand `git` or `date` a string with a named abbreviation like `MST`/`PST`. If no timezone is given, assume Mountain Time (`-0700` for MDT, Mar–Nov) and state which offset you used.

- **Empty input:** compute `cutoff` for 7 days ago and use the same `%ct` awk filter — do **not** fall back to `--since`.

Always exclude merge commits (`--no-merges`) and pipe through `| cat` / `cat -n` (git truncates long output by default).

**Step 2: Read the commits — bodies, not just subjects.**
Many commits here are squash-merges whose one-line subject hides what shipped (e.g. "Harden target-relative mass filter slots" or a bare PR title). Read the body of every non-obvious commit before writing its bullet — the body is where you learn whether a change is user-facing or internal, and which cards/mechanics it touches:
```bash
git show -s --format="%s%n%n%b" <hash>
```
Cross-check your understanding against the full commit count from Step 1 so nothing is silently dropped.

**Step 3: Synthesize into a grouped, user-facing changelog.**
- **Lead with a title line.** Start with `🎴 What's New in phase.rs`.
- **Group into emoji-headed categories.** Use sections, not a flat list. Typical sections (include only those with content, in this order):
  - `✨ New Cards & Mechanics` — new cards, keywords, or rules subsystems now playable
  - `🛠️ Cards That Now Work Right` — parser/engine fixes that make existing cards behave correctly
  - `⚔️ Combat & Gameplay` — combat, stack, priority, turn-structure fixes
  - `🖥️ Interface` — UI/UX changes
  - `🌍 Localization` — i18n
  - Add other emoji sections as the content warrants (e.g. `🤖 AI`, `🌐 Multiplayer`).
- **Bullets use `•`** and may run one or two lines. Consolidate multiple commits that build one feature into a single bullet — never mirror commits 1:1.
- **User-facing language only.** Describe what players can now do, not the implementation. "Eminence triggers now fire from the command zone" not "wire Eminence into command-zone trigger registry".
- **Name concrete cards/mechanics** in parentheticals when they clarify the change (e.g. "Karn, the Great Creator", "Angelic Arbiter").
- **Order by impact** within and across sections — most exciting / broadest-reach first.
- **Skip internal-only changes:** refactors, CI, feed refreshes, deploy/build wiring, semantic-audit/classifier tuning, code cleanup — unless they have user-visible impact.

**Step 4: Output the changelog.**
Present it in a **single fenced code block** so the user can copy-paste into Discord. The title line, emoji section headers, and `•` bullets all go inside the block. No preamble inside the block.

**Step 5: Footer (outside the code block) — enables the sequential workflow.**
After the code block, in normal prose, briefly:
- **List which commits you omitted and why** (e.g. "omitted the i18n deploy wiring and the semantic-audit classifier tune as internal-only"). This makes the editorial calls auditable.
- **State the new tip hash.** End with the current tip hash so a follow-up "show me the next changelog from your last batch" can use `<that-hash>..origin/main` as the next ref-range. Each batch's tip becomes the next batch's start boundary.
