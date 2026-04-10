# MCP GIF Recording Script

**Tool:** asciinema → agg → .gif
**Dimensions:** 100 cols × 20 rows (tighter — this is a shorter clip)
**Duration:** 12-15 seconds
**Speed:** 1.2x playback

---

## Recording sequence

```bash
asciinema rec mcp.cast --cols 100 --rows 20

# Step 1: Show the one-liner (type it out, don't paste — viewers like to see typing)
claude mcp add --transport stdio ctxforge -- ctxforge mcp

# Step 2: (If Claude Code outputs a success message, let it show)
# Pause 2s

# Step 3: Start a Claude Code conversation and show the agent using memory
claude

# In the Claude Code session, type:
# "use ctxforge to note that we decided to use tokio::select over join for cancellation"
# (Claude calls ctxforge_note tool → shows the tool call + result)

# Step 4: Show recall
# "what architectural decisions have we made?"
# (Claude calls ctxforge_recall → shows the notes)

# Pause 2s, then exit
```

## The key moments:

1. **The one-liner** (0-3s) — `claude mcp add` command typed out
2. **Agent writes a note** (5-10s) — Claude calls ctxforge_note, result shows
3. **Agent recalls memory** (10-15s) — Claude calls ctxforge_recall, notes come back

This GIF sells the "persistent memory in one command" narrative.
