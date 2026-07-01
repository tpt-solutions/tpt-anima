# TPT Anima — Demo Scenarios

> Copyright TPT Solutions · Apache License 2.0

This document describes three demo walkthroughs for the TPT Anima Observatory.

---

## Demo 1: Single-Agent Memory Lifecycle

### Overview
This demo shows how a single agent interacts with the Semantic State Engine (SSE) to store, retrieve, and decay memories.

### Steps
1. **Memory Ingestion**
   - Agent receives a text input: "The capital of France is Paris."
   - SSE converts the text to a vector embedding
   - A `MemoryNode` is created with:
     - `modality: Textual`
     - `strength: 1.0`
     - `causal_links: []`

2. **Memory Reinforcement**
   - Agent queries the same information: "What is the capital of France?"
   - `record_access()` is called on the memory node
   - Strength increases to `1.15` (reinforcement factor applied)

3. **Temporal Decay**
   - Time passes (simulated)
   - `apply_decay()` reduces strength based on elapsed time
   - After 24 hours: strength ~ `0.85`
   - After 7 days: strength ~ `0.45`

4. **Observatory Visualization**
   - Memory Topography shows the node with:
     - Initial bright glow (high strength)
     - Gradual dimming as decay progresses
     - Color indicates modality (text = indigo)

### Expected Output
- Memory node appears in topographical view
- Strength ring shows decay progress
- Causal chain visualization (if linked to other memories)

---

## Demo 2: Multi-Agent Intent Negotiation Swarm

### Overview
This demo demonstrates the Intent & Negotiation Protocol (INP) with multiple agents competing for compute resources.

### Setup
- **Agent Alpha**: Reasoner agent (high priority, low cost tolerance)
- **Agent Beta**: Executor agent (medium priority, balanced cost)
- **Agent Gamma**: Validator agent (low priority, high accuracy requirement)

### Steps
1. **Intent Broadcast**
   - Agent Alpha broadcasts: "Analyze Q3 financial report"
   - Intent includes:
     - `QoS: latency < 500ms, accuracy > 95%`
     - `EconomicTerms: PerUnit pricing`
     - `TTL: 30 seconds`

2. **Bid Collection**
   - Agent Beta responds: `cost=100, latency=200ms, accuracy=92%`
   - Agent Gamma responds: `cost=150, latency=400ms, accuracy=98%`
   - Agent Alpha self-bids: `cost=50, latency=100ms, accuracy=85%`

3. **Provider Selection**
   - CK evaluates bids against QoS constraints
   - Agent Gamma wins (meets accuracy requirement)
   - Agent Beta is fallback provider

4. **Execution & Monitoring**
   - ThoughtBubble allocated on GPU
   - Inference Pulse shows:
     - GPU utilization spike to 75%
     - VRAM usage increase
     - Thermal meter at 45%

5. **Result & Settlement**
   - Execution completes in 350ms
   - 150 Compute Credits transferred from Alpha to Gamma
   - Economic Flow animates credit transfer

6. **Swarm Map Visualization**
   - Pulse animation shows: Alpha → Gamma (green, won)
   - Fallback chain: Alpha → Beta (red, lost)
   - Node status updates: Gamma = busy, Alpha = active

### Expected Output
- Swarm Map shows intent flow with pulse animation
- Inference Pulse shows hardware utilization
- Economic Flow shows credit transfer
- Agency Ledger records the transaction

---

## Demo 3: Observatory Walkthrough

### Overview
This demo provides a guided tour of the Observatory UI, showing all four visualization panels.

### Steps
1. **Launch Observatory**
   - Start `anima-observatory` backend
   - Open `observatory-ui` in browser
   - WebSocket connection established

2. **Memory Topography Panel**
   - View memory nodes as glowing points
   - Observe causal chains as connecting lines
   - Watch temporal decay animation (nodes pulse and dim)
   - Modality colors:
     - Text: indigo (#818cf8)
     - Visual: violet (#a78bfa)
     - Audio: purple (#c084fc)

3. **Swarm Map Panel**
   - View agent network topology
   - Watch intent pulse animations
   - Legend shows: active (green), idle (yellow), busy (red)
   - Grid overlay for spatial reference

4. **Inference Pulse Panel**
   - Monitor hardware utilization bars (GPU, CPU, NPU, RAM)
   - Watch thermal/VRAM/FLOP meters
   - See ThoughtBubbles floating in allocation area
   - Real-time updates during inference

5. **Agency Ledger Panel**
   - View immutable audit log of ZK events
   - Filter by event type (session, delegation, attestation, etc.)
   - Toggle to Compliance Dashboard:
     - SOC2 Type II scores
     - HIPAA compliance metrics
   - See unique agent count and event statistics

6. **Economic Flow Panel**
   - View agent balance bars
   - Watch credit flow particles
   - See total circulation counter
   - Task cost breakdown by agent

### Expected Output
- All four panels render correctly
- Real-time telemetry updates
- Interactive filtering and toggling
- Compliance scores calculated correctly

---

## Running the Demos

```bash
# Terminal 1: Start backend
cd anima-observatory
cargo run

# Terminal 2: Start UI
cd observatory-ui
npm install
npm run dev
```

Navigate to `http://localhost:5173` to view the Observatory.