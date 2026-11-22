# Run checks for matching files

This example shows how Latch combines path and extension filters. It uses
Git for the checks, so there are no project dependencies to install.
Open `examples/filters` in a Git checkout of this repository.

## Try it in your agent

Start a new session in this directory with the [Latch plugin enabled](https://barakchamo.github.io/latch/docs/quickstart/#install-a-plugin).
Try one row at a time and restore your edits between turns.

| Ask the agent to edit | Checks at turn end |
| --- | --- |
| `docs/guide.md` | Git status, then Markdown whitespace |
| `src/message.ts` | Git status, then TypeScript whitespace |
| Both files in one turn | Git status, then both whitespace checks |
| `data.json` | Git status only |
| `README.md` | Git status only. The extension matches, but the path is outside `docs/`. |

To see a failure, ask the agent to add a line ending in two spaces to
`docs/guide.md`. The Markdown check identifies the line. Restore the file
afterward. If your agent doesn't show the report at turn end, see
[when its harness delivers failures](https://barakchamo.github.io/latch/docs/hooks/#where-reports-appear).

## Read the config

[`latch.json`](latch.json) always runs Git status on a changed turn. A
file must match **both** a path and an extension to trigger a whitespace
check. Each matching entry runs once, in order. The leading `**/` lets
the filters work inside this repository or when copied into a separate
Git project.

This example shows successful output so you can see which entries ran.
Passing checks are normally quiet. Use an agent session to test the
filters. A manual `latch run turn_end` runs every configured entry.
