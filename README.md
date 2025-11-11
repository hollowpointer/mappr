# mappr 🗺️

A simple network mapping and discovery tool.

## ⚠️ Requirements

* **Operating System:** Currently, `mappr` only works on **Linux** and **macOS**.
* **Rust:** You must have the [Rust toolchain](https://www.rust-lang.org/tools/install) (including `cargo`) installed to build the tool.

---

## 🛠️ Building from Source

At the moment, you must build `mappr` manually.

1.  **Clone the repository:**
    ```bash
    git clone [https://github.com/hollowpointer/mappr](https://github.com/hollowpointer/mappr)
    ```

2.  **Navigate into the project directory:**
    ```bash
    cd mappr
    ```

3.  **Build the release executable:**
    ```bash
    cargo build --release
    ```

4.  **Move to the target directory:**
    The binary will be located in `target/release`.
    ```bash
    cd target/release
    ```

---

## 🚀 Usage

You will likely need `sudo` (root privileges) for network discovery operations.

### Discover LAN

Scans the local area network.

* **Command:**
    ```bash
    (sudo) ./mappr discover lan
    ```
* **Short alias:**
    ```bash
    (sudo) ./mappr d lan
    ```

### Show Info

Displays information about your local network interfaces.

* **Command:**
    ```bash
    (sudo) ./mappr info
    ```
* **Short alias:**
    ```bash
    (sudo) ./mappr i
    ```
