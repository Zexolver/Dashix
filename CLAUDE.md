# Project Overview: "PocketServer" (Self-Hosted PaaS Dashboard)

## The Core Objective
I want to build a highly intuitive, visually appealing, and localized Platform-as-a-Service (PaaS) dashboard. This app will wrap complex, CLI-heavy backend networking tools into a simple, beautiful UI. It allows me to easily host websites, route dynamic services (like Docker apps/OxiCloud), run a mail server, and manage DNS settings across both Desktop (Host) and Android (Remote/Host via Shizuku).

## The Tech Stack
*   **Frontend UI:** Slint (Pure Rust, lightweight, native UI). Must support Slint's Material 3 themes, bridging dynamic system colors on Android.
*   **Backend Orchestrator:** Rust. A local daemon that manages configurations, detects network changes, and executes binaries.
*   **Android Bridge:** Shizuku / Root (`su`) API. Allows the Rust engine to run as a background service on Android, bypassing port restrictions.
*   **Dynamic DNS:** Rust (`reqwest` + `pnet`/`get_if_addrs`). A background cron-task syncing host IP/MAC with Dynv6 API for multiple subdomains.
*   **Edge Proxy (L4):** `rpxy-l4`. Routes raw TCP/UDP traffic (Minecraft, SSH, RustDesk, etc.).
*   **Web Proxy (L7):** `rpxy`. Routes HTTP/HTTPS traffic, handles Let's Encrypt / ACME certs automatically.
*   **Mail Server:** `stalwart-mail`. Rust-based all-in-one mail server (IMAP, JMAP, SMTP) with auto-spam filters.

## Core Architectural Rules
1.  **Separation of Concerns:** The UI (Slint) must be decoupled from the backend orchestrator. They should communicate via a local API or IPC.
2.  **Auto-Detection:** The user should *never* have to manually input MAC or IPv6 addresses. The Rust backend must auto-detect available network interfaces (e.g., `eth0`, `wlan0`), allowing the user to select the interface from a dropdown, and automatically bind the correct IPs.
3.  **Portability:** The Rust backend must compile to standard desktop targets AND `aarch64-linux-android`.

## Dashboard UI Modules
The Slint UI should abstract technical jargon into four clear modules:

1.  **The Gateway (Network & DNS):**
    *   Toggle to enable Dynv6 syncing.
    *   List/UI to add multiple Dynv6 subdomains.
    *   Dropdown to select network interface (auto-grabs MAC/IPv6).
    *   Real-time connection/sync status indicator.
2.  **The App Router (Reverse Proxy & Web):**
    *   Visual router: Connect `[subdomain]` -> to `[Static Folder]` OR `[Local Port]`.
    *   Folder picker for static hosting + "Hot Reload" toggle.
    *   Local port input for dynamic routing (e.g., pointing a subdomain to an OxiCloud Docker container).
    *   One-click SSL/TLS toggles (updates `rpxy` ACME configs).
3.  **The Security Shield (rpxy-l4 edge):**
    *   Visual dashboard showing traffic splitting between L4 (TCP/UDP), L7 (Web), and Mail.
    *   Toggles to block specific IPs or route raw SSH/Minecraft ports.
4.  **The Post Office (Stalwart Mail):**
    *   Simplified setup wizard adjusting Stalwart TOML configs.
    *   UI to add/remove email user accounts.
    *   Status indicators for Mail Server health.

## Step-by-Step Implementation Plan
Please implement this project sequentially in the following phases. **Wait for my confirmation before moving to the next phase.**

### Phase 1: The Rust Orchestrator & Config Engine
*   Create the core Rust daemon.
*   Implement the network interface scanner (fetching MAC/IPv6).
*   Implement the Dynv6 update loop (for multiple domains).
*   Create the configuration generators for `rpxy`, `rpxy-l4`, and `stalwart`.
*   *Do not write the UI yet.* Focus on establishing a local API/Socket that the UI will eventually consume.

### Phase 2: The Slint Desktop UI
*   Initialize the Slint project with the Material 3 theme.
*   Create the 4 main modules (Gateway, App Router, Security Shield, Post Office).
*   Hook the UI up to the Phase 1 Rust daemon API.
*   Ensure the dynamic routing (Port vs. Static Folder) works properly in the App Router UI.

### Phase 3: Android Adaptation & Shizuku
*   Implement the Android wrapping code for Slint.
*   Implement the bridge to pull Android 12+ dynamic system colors and inject them into Slint.
*   Add the Shizuku / Root (`su`) execution layer so the Rust daemon from Phase 1 can run natively on the Android device as a host.
