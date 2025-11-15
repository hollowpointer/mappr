# mappr 🗺️

Easy to use network mapping and discovery tool.

<img width="1200" height="720" alt="mappr" src="https://github.com/user-attachments/assets/05722b01-cf9f-4820-9fd0-7c53f22928ee" />


## ⚠️ Requirements

* **Operating System:** Currently, `mappr` only works on **Linux** and **macOS**.
* **Rust:** You must have the [Rust toolchain](https://www.rust-lang.org/tools/install) (including `cargo`) installed to build the tool.

---

## 🛠️ Building from Source

At the moment, you must build `mappr` manually.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/hollowpointer/mappr
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

> **Heads-Up:** Network discovery operations (`discover`) typically require root privileges. You will likely need to prefix these commands with `sudo`.

### Core Commands

Here's a quick overview of the main commands (much more will be added soon):

| Command | Alias | Description |
| :--- | :--- | :--- |
| `mappr discover <target>`| `mappr d <target>` | Scans a specific, user-defined target or range (see below). |
| `mappr info` | `mappr i` | Displays info about your local network interfaces. |

---

### Host Discovery

You can discover hosts in two main ways.

**1. Automatic LAN Scan**

  * Automatically finds and scans your local network based on your computer's current IP address and subnet.
  * **Command:**
    ```bash
    mappr d lan
    ```

**2. Specific Target Scan**

  * Manually define a specific target or range to scan. This is accepted in three flexible formats:

      * **CIDR Notation:** Scans the entire subnet.

        ```bash
        mappr d 10.0.0.0/24
        ```

      * **Full IP Range:** Scans all IPs between the two addresses.

        ```bash
        mappr d 172.16.0.1-172.16.0.254
        ```

      * **Partial Octet Range (Shorthand):** A convenient shortcut where `mappr` fills in the blanks from the first IP.

        *Example 1 (Last octet):*

        ```bash
        # Expands to 192.168.0.1-192.168.0.50
        mappr d 192.168.0.1-50
        ```

        *Example 2 (Multiple octets):*

        ```bash
        # Expands to 10.0.0.1-10.1.2.3
        mappr d 10.0.0.1-1.2.3
        ```

---

### Show Info

Displays information about your network configuration.

* **Command:**
    ```bash
    mappr info
    ```
* **Short alias:**
    ```bash
    mappr i
    ```
