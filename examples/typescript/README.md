# TypeScript

Run these commands from this directory.

```sh
npm ci
latch explain after_edit
latch run after_edit
latch run turn_end
```

Latch discovers both scripts in `package.json` and uses npm because this
example has a `package-lock.json`. Both checks are silent when they pass.
Change `greeting` to return a number to see the type-check diagnostic.
