# Hero GIF Recording Script

**Tool:** asciinema → agg → .gif (or asciinema → svg → gif via svg-term-cli)
**Dimensions:** 120 cols × 35 rows (renders well at 1200×700 in GIF)
**Duration:** 10-12 seconds
**Speed:** 1.5x playback (makes it feel snappy)

---

## Setup before recording

```bash
# Create a demo project directory
mkdir -p /tmp/ctxforge-demo/src/hub
cat > /tmp/ctxforge-demo/src/hub/server.go << 'EOF'
package hub

import (
    "net/http"
    "crypto/tls"
)

func StartServer(addr string, config *tls.Config) error {
    srv := &http.Server{
        Addr:      addr,
        TLSConfig: config,
    }
    return srv.ListenAndServeTLS("", "")
}

func handleRequest(w http.ResponseWriter, r *http.Request) {
    // process incoming request
    w.WriteHeader(http.StatusOK)
}
EOF

cat > /tmp/ctxforge-demo/src/hub/check.go << 'EOF'
package hub

import "fmt"

func ProcessCheck(c *Check) error {
    if c == nil {
        return fmt.Errorf("nil check")
    }
    // validate check parameters
    if c.Interval < 1 {
        return fmt.Errorf("interval must be >= 1")
    }
    return nil
}

type Check struct {
    Name     string
    Interval int
    Target   string
}
EOF

cat > /tmp/ctxforge-demo/README.md << 'EOF'
# WatchDog Hub

A monitoring system that watches services behind firewalls.

## Architecture

The Hub receives check results from distributed agents via WebSocket.
Agents run inside private networks and connect outbound — no inbound
firewall rules needed.

## Getting Started

1. Configure the hub with your database credentials
2. Deploy agents to each network you want to monitor
3. Create monitors in the dashboard

See docs/architecture.md for the full system design.
EOF

cat > /tmp/ctxforge-demo/docs/architecture.md << 'EOF'
# Architecture

## Overview

WatchDog uses a hub-and-spoke architecture. The hub is the central
coordination point. Agents are lightweight binaries deployed inside
target networks.

## Communication

All communication is agent-initiated over WebSocket. This means:
- No inbound firewall rules needed
- No VPN tunnels required
- No port forwarding
- Agents work behind NAT

## Data Flow

1. Hub pushes check configurations to connected agents
2. Agents execute checks locally
3. Results stream back over the same WebSocket connection
4. Hub processes results, manages incidents, sends alerts

## Database

PostgreSQL with TimescaleDB for time-series check results.
Connection pooling via pgx. Migrations managed with golang-migrate.

## Authentication

- User auth: Argon2id password hashing
- Agent auth: API keys with AES-256-GCM encryption at rest
- API tokens: SHA-256 hashed, scoped (admin/read-only)
EOF

# Clear any existing ctxforge state
cd /tmp/ctxforge-demo
rm -rf .ctxforge
```

## Recording sequence

```bash
# Start recording
asciinema rec hero.cast --cols 120 --rows 35

# Step 1: Launch TUI (pause 1s to let viewer see it appear)
ctxforge

# Step 2: In the TUI:
#   - Press j/j/j to move down to server.go
#   - Press space (token gauge jumps — visible!)
#   - Press j to check.go
#   - Press space (gauge grows more)
#   - Press j/j to README.md
#   - Press space (gauge grows — now yellow-green)
#   - Press j to architecture.md
#   - Press space (gauge jumps noticeably — this is the hotspot moment)
#   - Pause 1.5s so viewer sees the hotspot highlight
#   - Press c (status bar shows "Copied X items to clipboard")
#   - Pause 1s
#   - Press q to quit

# End recording
# (asciinema auto-ends when you exit)
```

## Post-processing

```bash
# Convert to GIF (using agg — asciinema gif generator)
agg hero.cast hero.gif --speed 1.5 --theme monokai

# Or using svg-term-cli + svg2gif
svg-term --in hero.cast --out hero.svg --window --no-cursor
# Then convert SVG to GIF with your preferred tool
```

## The 3 moments viewers must see:

1. **TUI appears** (0-2s) — the two-panel layout looks polished
2. **Token gauge fills** (2-8s) — visible, color-graded response to each toggle
3. **Hotspot highlight** (6-8s) — architecture.md lights up orange at >25%
4. **Copy confirmation** (8-10s) — "Copied 4 items (4,365 tokens) to clipboard"

If these three moments don't land clearly in the GIF, re-record.
