# ctxforge Launch Checklist

## T-minus 2 weeks

- [ ] Record hero GIF (TUI with token gauge) using hero-gif-script.md
- [ ] Record MCP GIF using mcp-gif-script.md
- [ ] Record vertical reel using instagram-reel-script.md
- [ ] Create carousel images using carousel-slides.md as content guide
- [ ] Finalize Medium article (medium-article.md) — publish to Better Programming / Level Up Coding
- [ ] Set up linktree/bio with GitHub repo link
- [ ] Tease on Instagram: 10s clip of just the token gauge filling, caption "Building something."
- [ ] Tease on LinkedIn: "Spent the last 4 weekends on a side project. Shipping next Tuesday."

## T-minus 1 week

- [ ] Ship to ~10 private alpha testers (Claude Code / Cursor power users)
- [ ] Collect one-sentence testimonials for social proof
- [ ] Fix any bugs from alpha testing
- [ ] Upload asciinema recordings (unlisted) to asciinema.org
- [ ] Finalize all recording assets in their final form
- [ ] Prep all post drafts (don't publish yet)

## Launch Day (Tuesday, staggered)

### 9:00 AM — GitHub
- [ ] Make repo public: `gh repo edit sylvester-francis/ctx-forge --visibility public --accept-visibility-change-consequences`
- [ ] Verify badges resolve (give shields.io 2-3 minutes)
- [ ] Create GitHub Release for v0.7.0 with hero GIF + CHANGELOG entry
- [ ] Pin a Discussion: "Show us your ctxforge workflow"
- [ ] Enable Sponsors (optional)

### 9:30 AM — Medium
- [ ] Publish the article (medium-article.md)
- [ ] Submit to publications: Better Programming, Level Up Coding, ITNEXT
- [ ] Tags: artificial-intelligence, programming, rust, developer-tools, productivity

### 10:00 AM — LinkedIn
- [ ] Post (linkedin-post.md) with hero GIF as native video/image
- [ ] Immediately add Comment 1 (GitHub link), Comment 2 (Medium link), Comment 3 (crates.io)

### 10:30 AM — Instagram
- [ ] Post reel (instagram-reel-script.md)
- [ ] Post carousel (carousel-slides.md)
- [ ] Add Stories with link sticker to repo
- [ ] Update bio with GitHub link

### 11:00 AM — Threads
- [ ] Post thread (threads-thread.md) — all 7 posts in sequence
- [ ] Reply tagging @anthropic, @cursor_ai, @rustlang

## Day 3 (Thursday) — Hacker News bonus

- [ ] Post Show HN (show-hn.md) at 9-10 AM EST
- [ ] Reply to every comment within 2 hours
- [ ] Cross-reference the Medium article in replies when relevant

## T+1 → T+7 — Compound phase

- [ ] Reply to every comment across all channels within 2 hours
- [ ] Repost reel on day 3 with different caption if performing
- [ ] LinkedIn republish Medium article natively on day 3
- [ ] New Instagram reel on day 5 (single feature: hotspot panel or fn extraction)
- [ ] Daily Threads post: one small insight per day
- [ ] Respond to every GitHub issue and PR within 24 hours
- [ ] Cross-post to: r/rust, r/ClaudeAI, r/cursor, r/ChatGPTCoding
- [ ] Submit to: Rust Weekly, Bytes newsletter, TLDR newsletter

## T+14 — Phase 2 wave

- [ ] Publish Medium article #2: "How I Gave Claude Code Persistent Memory in 300 Lines of Rust"
- [ ] New Instagram reel: "The MCP trick that changes everything"
- [ ] LinkedIn post: career-framing, "What I learned shipping an open-source tool to X stars"
- [ ] Threads: live-thread the Phase 2 article insights

## Content cadence post-launch (every 2-3 weeks)

Each article → 1 LinkedIn post + 1 Instagram carousel + 1 Threads thread:

1. T+14: "Giving Claude Code persistent memory (MCP deep-dive)" — Write pillar
2. T+30: "Tree-sitter in 200 lines of Rust" — Select pillar
3. T+45: "The token gauge: why percentages beat raw counts" — Compress pillar
4. T+60: "Shared context profiles: the editorconfig for AI" — Isolate pillar
5. T+75: "Building a ratatui TUI people actually enjoy" — craft / Rust community
