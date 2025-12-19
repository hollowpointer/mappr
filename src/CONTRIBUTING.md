# Contributing to Mappr

Welcome to the **Mappr** development guide. This document explains our unique architecture, where to find things, and how to add features without breaking the specific design patterns we use.

## 🏗 Operations: The Hybrid Architecture

Mappr uses a **Hybrid Engine/Shell Architecture**. This is distinct from a pure "Clean Architecture" or a pure "Scripting" approach.

We have two distinct worlds with different rules:

### 1. The Shell (Safe, Clean, Maintainable)
**Locations**: `src/domain`, `src/application`, `src/ports`, `src/adapters`

The "Shell" wraps the application. It handles user input (CLI), configuration, and high-level orchestration. It follows strict **Hexagonal Architecture (Ports and Adapters)**.

*   **Goal**: Maintainability, Testability, Stability.
*   **Rules**: 
    *   **Dependency Rule**: Dependencies point INWARD. `adapters` -> `application` -> `domain`.
    *   **No Infrastructure**: The `domain` layer must NOT depend on `pnet`, `tokio` (runtime), or OS-specific calls.
    *   **Rich Models**: Use strictly typed structs (`Host`, `IpServiceGroup`).

### 2. The Engine (Fast, Raw, Dangerous)
**Location**: `src/engine`

The "Engine" is the high-performance core that talks to the network. It prioritizes speed and low-level control over architectural purity.

*   **Goal**: Raw Performance, Zero-Copy, Batch Processing.
*   **Rules**:
    *   **Anything Goes**: You can use `pnet`, raw pointers, optimization hacks, and OS syscalls.
    *   **Internal Types**: Use optimized internal structs (e.g., `EngineHost`) to save memory/cycles in hot loops.
    *   **Shared Types**: The `engine` MAY depend on `domain::models` to share common types (e.g., `Host`, `Ipv4Range`) and avoid redundant definitions.

---

## 🗺 Directory Map

| Path | Role | Key Rules |
| :--- | :--- | :--- |
| **`src/domain`** | The "Truth". Data structs & Logic. | Pure Rust only. No external deps. |
| **`src/application`** | Services & Use Cases (`Services`). | Orchestrates `ports`. No IO. |
| **`src/ports`** | Interfaces (`Traits`) for IO. | Defines *what* we need, not *how*. |
| **`src/adapters`** | Implementations of Ports. | Where `main` logic meets the real world. |
| **`src/engine`** | **The Core**. `protocol`, `datalink`. | Fast. Dirty. Isolated. |

---

## 👩‍💻 How-To Guides

### "I want to add a new Network Protocol (e.g., mDNS)"

1.  **Go to `src/engine/protocol`**: Create `mdns.rs`.
2.  **Implement Logic**: Write functions to serialize/deserialize packets using `pnet` or raw bytes.
3.  **Update `src/engine/runner/local.rs`**: Add a hook in the hot-loop (`process_udp_packets`) to handle mDNS traffic.
4.  **Update `EngineHost`**: If mDNS discovers new info (like a hostname), update `src/engine/models.rs`.
5.  **Expose to Shell**: Update `NetworkScannerAdapter` to genericize the new data into the `Host` domain model.
6.  **Done**: The CLI gets the data for free because it consumes `Host`.

### "I want to add a new CLI Command"

1.  **Go to `src/adapters/inbound/cli`**: Create your command module.
2.  **Define Intent**: Does this need a new Service?
    *   *Yes*: Create `src/application/services/my_service.rs`.
    *   *No*: Reuse existing services.
3.  **Wire it up**: In `src/bin/mappr.rs`, add the command parsing and instantiate the service.

### "I want to refactor a Core entity"

1.  **Go to `src/domain`**: Modify the struct.
2.  **Fix Ripples**: The compiler will tell you where `adapters` break.
3.  **Check ACL**: You might need to update the mapping logic in `ScannerAdapter` where `EngineHost` converts to `InternalHost`.

---

## 🚦 Do's and Don'ts

| ✅ DO | ❌ DON'T |
| :--- | :--- |
| **Map types at the boundary** | Pass `pnet::NetworkInterface` into `src/domain`. |
| **Keep `domain` pure** | Add `#[cfg(target_os = "linux")]` in `src/domain`. |
| **Hack performance in `engine`** | Write "clever" unreadable code in `src/application`. |
| **Use `anyhow` in Adapters** | Panic or wrap errors in `domain` (return `Result`). |
| **Create new Ports** for logic | Call `SystemRepo` directly from `DiscoveryService`. |

## 🧪 Testing

*   **Unit Tests**: Write them next to the code.
*   **Engine Tests**: Critical. Test protocol parsers with mock bytes.
*   **Integration**: Run `cargo run -- discover <target>` to verify the full stack.
