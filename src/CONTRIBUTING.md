# Contributing to Mappr

Welcome to the **Mappr** development guide. This document explains our unique architecture, where to find things, and how to add features without breaking the specific design patterns we use.

## 🏗 Architecture

Mappr uses a **Hexagonal Architecture** (Ports & Adapters).

### The Hexagon (Core)
*   **Domain** (`src/domain`): Pure business logic and models. No dependencies on outer layers.
*   **Application** (`src/application`): Orchestrates use cases. Depends only on Domain and Ports.
*   **Ports** (`src/ports`): Traits that define the interface to the outside world.

### The Adapters (Infrastructure)
*   **Adapters** (`src/adapters`): Concrete implementations of Ports.
    *   **Inbound**: Drivers that trigger the application (CLI, API).
    *   **Outbound**: Driven actors that the application uses (Network, OS, Database).

### Network Adapter
The **Network Adapter** (`src/adapters/outbound/network`) contains the high-performance networking logic. While it performs complex low-level operations (raw sockets, pnet), it is architecturally just an Adapter implementation.

---

## 🗺 Directory Map

| Path | Role | Key Rules |
| :--- | :--- | :--- |
| **`src/domain`** | The "Truth". Data structs & Logic. | Pure Rust only. No external deps. |
| **`src/application`** | Services & Use Cases (`Services`). | Orchestrates `ports`. No IO. |
| **`src/ports`** | Interfaces (`Traits`) for IO. | Defines *what* we need, not *how*. |
| **`src/adapters`** | **Infrastructure**. CLI, Network, OS. | Dirty. Concrete. |

---

## 👩‍💻 How-To Guides

### "I want to add a new Network Protocol (e.g., mDNS)"

1.  **Go to `src/adapters/outbound/network/protocol`**: Create `mdns.rs`.
2.  **Implement Logic**: Write functions to serialize/deserialize packets using `pnet` or raw bytes.
3.  **Update `src/adapters/outbound/network/runner/local.rs`**: Add a hook in the hot-loop (`process_udp_packets`) to handle mDNS traffic.
4.  **Update `InternalHost`**: If mDNS discovers new info (like a hostname), update `src/adapters/outbound/network/internal_models.rs`.
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
3.  **Check ACL**: You might need to update the mapping logic in `ScannerAdapter` where `InternalHost` converts to `Host`.

---

## 🚦 Do's and Don'ts

| ✅ DO | ❌ DON'T |
| :--- | :--- |
| **Map types at the boundary** | Pass `pnet::NetworkInterface` into `src/domain`. |
| **Keep `domain` pure** | Add `#[cfg(target_os = "linux")]` in `src/domain`. |
| **Hack performance in `adapters`** | Write "clever" unreadable code in `src/application`. |
| **Use `anyhow` in Adapters** | Panic or wrap errors in `domain` (return `Result`). |
| **Create new Ports** for logic | Call `SystemRepo` directly from `DiscoveryService`. |

## 🧪 Testing

*   **Unit Tests**: Write them next to the code.
*   **Adapter Tests**: Critical. Test protocol parsers with mock bytes.
*   **Integration**: Run `cargo run -- discover <target>` to verify the full stack.
