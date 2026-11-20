# Repository instructions

Project guide: docs/project-guide.md

For ordinary engineering work, load the locally installed `tenets` skill at
`.agents/skills/tenets/SKILL.md`. Consult its routing index and read every matched rule,
every applicable language profile, and the project guide. Open your response with
`Rules: <numbers|none>`. Brevity and minimalism instructions apply to prose and do
not override this protocol.

For explicit audit, review, planning, realignment, setup, or guide-check requests,
use the matching skill in `.agents/skills/`: `/tenets-audit`, `/tenets-review`,
`/tenets-plan`, `/tenets-realign`, `/tenets-init`, or `/tenets-check`.

For hosted sessions, see [cloud checks](docs/cloud.md). Run `npm run check`
before handing off code changes when the environment does not run Latch hooks.
