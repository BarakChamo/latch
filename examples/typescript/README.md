# TypeScript with npm

Use Node.js 22.12 or newer. Run these commands from this directory.

```sh
npm ci
latch explain turn_end
latch run after_edit
latch run turn_end
```

Latch finds `hook:afteredit` and `hook:turnend` in `package.json` and uses
npm because the example has a `package-lock.json`. Both checks are quiet
when they pass.

Change `Hello` to `Hi` in [`src/message.ts`](src/message.ts) and run
`latch run turn_end`. The greeting still type checks, but Vitest reports
the expected and actual messages and Latch exits with status 1. Restore
`Hello` to make the check pass again.
