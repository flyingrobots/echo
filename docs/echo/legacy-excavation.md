# Echo Legacy Excavation Log

| File | Role (2013) | Verdict | Roast | Notes & Action Items |
| --- | --- | --- | --- | --- |
| README.md | Project teaser, submodule manifest dump | Rescope | “Sweet stuff coming soon” has been on break for 12 years. | Replace with Echo overview. Document historical submodules in appendix. |
| docs/roadmap.md | Milestone wish list | Inspire | Athens never fell, but at least the ambition was mythic. | Retain naming spirit; extract goals relevant to Echo tooling. |
| docs/guidelines.md | Team philosophy, workflow tips | Archive | All gas, no brakes, and Mootools forever? Adorable. | Capture cultural nuggets (automation emphasis), discard mootools bias. |
| docs/notes.md | Scratchpad of ideas/links | Discard | Mou links and sweet hacker vibes—time capsule material only. | Note links of lasting value (Sinon, Vows) elsewhere if needed. |
| server/src/app.js | Express 3 static file server | Replace | Global `p` variables like it’s frontier JavaScript. | Build new dev server using modern tooling (Vite/Express 5). No direct migration. |
| server/config/*.json | Environment roots for sandbox | Replace | Two configs, zero validation, still yelling into port 1337. | Echo config moves to typed bootstrap pipeline; reuse concept of content roots. |
| client/index.html | Legacy client bootstrap | Replace | Script tags summon 2012 directly into your DOM. | Reference for dependency list; future sample uses modern bundler. |
| client/src/engine/component.js | Component registration via global | Reimagine | “FIXME: does not appear to work??” is the documentation. | Preserve idea of type IDs, but implement through registration metadata in TypeScript. |
| client/src/engine/entity.js | Entity class with signal events | Reimagine | Signals galore, but `onRemoved` can’t find `components`. | Convert to pooled entity manager; retain add/remove events. Note bug in `onRemoved`. |
| client/src/engine/utils.js | Misc helpers (`safeInvoke`, `randomColor`) | Replace | `js.defaults` is promised but never born. | Identify useful helpers (`safeInvoke`, `insertWhen`) and decide whether to reintroduce or drop; modern JS/TS covers most needs. |
| client/src/engine/typed_json.js | JSON revive by type string | Reimagine | `stringToFunction` must be hiding with Half-Life 3. | Document requirement for typed serialization; implement schema-driven version. |
| client/src/engine/system.js | System registration, node list management | Reimagine | ECS via existential dread and `FIXME` confessions. | Keep node concept but redesign around archetypes; note missing `for...` guard bugs. |
| client/src/engine/system_node_list.js | Query builder | Replace | `for (var node in nodes)`—so close to working. | Use signature-driven queries; keep idea of node add/remove signals. |
| client/src/engine/world.js | Entity storage, prefab creation | Reimagine | Mootools `.extend` shows up like it’s a dependency. | Avoid Mootools dependencies; plan new world/scene API with deterministic removal. |
| client/src/engine/game.js | Game loop and state machine placeholder | Replace | Everything interesting lives in commented-out physics demo. | Use scheduler + state stack; record expectation for physics warm-up, pause, debug toggles. |
| client/src/engine/state_machine.js | Simple state transitions | Replace | Calls `setup` with `game` global that never existed. | Keep concept of safe enter/exit; remove global `game` usage. |
| client/src/engine/system_registry.js | Global system manager | Replace | Pauses by flag, but erases systems with `erase` like it’s Mootools. | Merge into scheduler design; note priority + pause semantics worth preserving. |
| client/src/engine/input.js | DOM event wrappers | Replace | Manual event wiring with `document.onkeydown`—no mercy for multiple instances. | Build Input port with adapter for browser; note features (callbacks, unregister). |
| client/src/game/** | Early game-specific systems/components | Optional | Kinetics component remembers a dream of box2d glory. | Use select pieces (kinetics, player input) as examples or tests; majority deprecated. |
| sandbox/** | Experiments and demos | Archive | Tumbleweed demos and a `stub.todo` note to self. | Inventory useful samples; otherwise treat as historical artifacts. |
| tasks/*.rake | Automation scripts (node install, pixi copy) | Replace | Rake calling `sudo npm install` feels like a dare. | Modern dev workflow handled via package scripts; keep spirit of automation (one-command setup). |

*Legend*:  
**Replace** – Concept stays but implementation rewritten.  
**Reimagine** – Core idea adapted with significant evolution.  
**Archive** – Preserve in history docs but not implemented.  
**Discard** – No longer relevant; drop entirely.  
**Inspire** – Keep stylistic or cultural notes, not functionality.
