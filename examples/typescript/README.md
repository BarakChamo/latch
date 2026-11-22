# TypeScript with npm

Latch discovers the commands in [`package.json`](package.json). Prettier runs after edits. TypeScript and Vitest run at turn end. The project uses the npm lockfile and needs no `latch.json`.

## Run the checks

Use Node.js 22.12 or later and [install the Latch CLI](https://barakchamo.github.io/latch/docs/cli/#install-the-cli). Run these from this directory.

```sh
npm ci
latch run after_edit
latch run turn_end
```

## See a failing check

Change `Hello` to `Hi` in [`src/message.ts`](src/message.ts), then run
`latch run turn_end`. TypeScript still passes, but Vitest shows the expected
and actual greetings. Latch exits with status 1. Restore `Hello` to pass again.

## Try automatic feedback

Start a new session in this directory with the [Latch plugin enabled](https://barakchamo.github.io/latch/docs/quickstart/#install-a-plugin)
and ask the agent to make the same edit. [When your agent sees failures](https://barakchamo.github.io/latch/docs/hooks/#where-reports-appear)
