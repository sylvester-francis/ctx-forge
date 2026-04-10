# Instagram Carousel — "Four Pillars of Context Engineering"

**Format:** 6 slides, 1080x1350, consistent brand colors (dark bg, cyan/green accents)

---

## Slide 1 — HOOK

**Large text:** "Your AI agent keeps forgetting everything."

**Subtext:** "Here's how to fix it."

**Small text at bottom:** ctxforge — context engineering CLI

---

## Slide 2 — WRITE (Pillar 1)

**Heading:** WRITE — Persistent Memory

**Body:**
```
ctxforge note --tag auth "JWT in header"
ctxforge recall --tag auth
ctxforge resume
```

**Subtext:** "Your agent writes decisions to itself. Reads them back next session."

**Visual:** Small icon of a notebook/pen

---

## Slide 3 — SELECT (Pillar 2)

**Heading:** SELECT — Surgical Precision

**Body:**
```
ctxforge add src/**/*.rs
ctxforge add --fn ProcessCheck src/check.go
ctxforge add --diff main
```

**Subtext:** "Add files, line ranges, specific functions. You choose what the agent sees."

**Visual:** Small icon of a crosshair/target

---

## Slide 4 — COMPRESS (Pillar 3)

**Heading:** COMPRESS — See What You're Spending

**Body:** [Mockup of the TUI token gauge]

```
tokens ████████░░░░  4,365 / 200,000 (2.2%)
```

**Subtext:** "Live token gauge. Hotspot alerts. Know which file is eating your budget."

**Visual:** Gauge bar in green → yellow gradient

---

## Slide 5 — ISOLATE (Pillar 4)

**Heading:** ISOLATE — Clean Separation

**Body:**
```
ctxforge save watchdog-hub
ctxforge load dkl-core
ctxforge profiles
```

**Subtext:** "Profiles keep work streams separate. No context contamination across tasks."

**Visual:** Small icon of compartments/boxes

---

## Slide 6 — CTA

**Large text:** "cargo install ctxforge"

**Subtext:** "Rust CLI. No cloud. No API keys. AGPL-3.0 open source."

**Smaller text:**
- GitHub: github.com/sylvester-francis/ctx-forge
- Full article: link in bio

**Visual:** ctxforge ASCII art logo
