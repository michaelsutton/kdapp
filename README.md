# Kdapp

A framework for building high-frequency, interactive decentralized applications on the Kaspa blockDAG.

> **⚠️ Alpha Software**
> This project is in an early development phase. The API is not yet stable and the codebase should be considered experimental. Expect breaking changes as the framework evolves.

-----

## Overview

Kdapp provides the infrastructure to build semi-native, interactive, and time-sensitive decentralized applications (k-dApps) on Kaspa. These interactive sessions are called **"Episodes."**

The project's philosophy is to provide the fastest possible route for developers to build and deploy creative applications that leverage Kaspa's unique 10 blocks-per-second capability. It prioritizes speed of development and on-chain performance, accepting initial trade-offs in areas like state persistence and interoperability.

This approach allows for novel use cases that depend on a high-frequency clock, such as on-chain blitz chess, real-time sports betting fed by oracles, or other interactive, multi-participant protocols. The long-term vision is to streamline development to the point where entire episodes can be generated through AI-assisted "vibe coding" from high-level prompts.

-----

## How It Works

The framework's architecture is designed for efficiency and modularity:

1.  **`Generator`**: A utility that crafts Kaspa transactions with specially formatted payloads. It seeks a transaction ID matching a predefined pattern, allowing for highly efficient discovery of episode-related transactions on the network.
2.  **`Proxy`**: A wRPC client that listens to the Kaspa network specifically for transactions matching the generator's pattern. Valid commands are then forwarded to the core engine.
3.  **`Engine`**: The central controller that manages the lifecycle of multiple episodes of the same type. It interprets incoming commands, validates signatures, updates episode state, and maintains a stack of rollback objects to handle Kaspa DAG re-organizations.
4.  **`Episode` & `EpisodeEventHandler`**: The primary developer interfaces. You implement the `Episode` trait to define your application's state and command logic. A corresponding `EpisodeEventHandler` trait allows for injecting logic to track episode progress and report state changes to clients.

This creates a clear data flow:
`Developer Implements Episode` → `Generator Creates TX` → `Proxy Hears TX` → `Engine Executes Command` → `Event Handler Reports`

-----

## Getting Started: Running the Tic-Tac-Toe Example

This repository includes a fully functional Tic-Tac-Toe example that demonstrates how two players can execute a complete game on the Kaspa network.

#### Prerequisites

  * **Rust Toolchain**: [Install Rust](https://www.rust-lang.org/tools/install).
  * **Kaspa Node Access**: A wRPC connection is required. The app defaults to public [PNN](https://kaspa.aspectron.org/rpc/pnn.html) nodes, but you can specify your own, for example: `--wrpc-url wss://localhost:17210`.

#### Step 1: Build the Example

Clone the repository and build the `ttt` binary:

```bash
git clone https://github.com/michaelsutton/kdapp.git
cd kdapp
cargo build --release --bin ttt
```

#### Step 2: Generate and Fund a Kaspa Address

Both players must have a funded Kaspa address to pay for transaction fees. The application can generate one for you.

1.  Run the application with no arguments to generate a new keypair:
    ```bash
    ./target/release/ttt
    ```
2.  The program will output a Kaspa address and a private key. Send some testnet Kaspa (TKAS) to the address. You can get testnet funds from the [Kaspa Faucet](https://www.google.com/search?q=https://faucet.kaspanet.io/).

#### Step 3: Player 1 (Starts the Session)

1.  Run the application again, providing your funded private key:
    ```bash
    ./target/release/ttt --kaspa-private-key <your-kaspa-private-key>
    ```
2.  The application will generate and display a new **Game Public Key**. Copy this key and send it to Player 2 (e.g., via a messaging app).
3.  Your terminal will now wait for the opponent to join and initiate the game.

#### Step 4: Player 2 (Joins and Initiates the Game)

After funding your own Kaspa address, run the application with your private key **and** the game key you received from Player 1. This command will start the on-chain game.

```bash
./target/release/ttt --kaspa-private-key <your-kaspa-private-key> --game-opponent-key <player-1-game-key>
```

#### Step 5: Play the Game

Once the game starts, both players' terminals become interactive. When prompted, enter your move in `row,col` format (e.g., `1,1` for the center square). The game runs on `testnet-10` by default; add the `--mainnet` flag to use mainnet instead.

-----

## Future Directions & Starting Points

This is a community-driven framework. The best way to contribute is to fork the repository and take the project in new and unexpected directions (either tailored for specific apps or in general form). Use the list below for inspiration, or bring your own unique ideas to the framework.

-----

#### **Good First Issues**

  * `[ ]` **Implement In-Memory Rollback Cap:** The `engine` holds all rollback objects in memory. A well-contained first contribution would be to implement a simple cap on this rollback stack (e.g., keep the last 1,000 entries per episode) to prevent memory exhaustion.
  * `[ ]` **Create a Pattern Utility Function:** Add a utility to generate a deterministic transaction ID pattern from a unique string prefix, allowing for human-readable or branded transaction streams.
  * `[ ]` **Expand Code Documentation:** Improve the `rustdoc` comments throughout the codebase, particularly in the `engine`, `proxy`, and `generator` modules, to clarify internal logic for new developers.
  * `[ ]` **Create Additional Examples:** Implement another slightly more complicated `Episode` example to further demonstrate the framework's use and provide another reference for developers.

#### **Core Engine & Infrastructure**

  * `[ ]` **Optimize RPC Syncing for High Throughput:** Improve the RPC syncing process to handle heavy DAG load. This is a significant task that might involve replacing the current polling mechanism with VSPC notifications, using the `get_blocks`/`block notifications` RPC API to prefetch block data, or contributing an RPC extension to Rusty-Kaspa for more efficient transaction fetching.
  * `[ ]` **Design a Decoupled Client-Server Architecture:** To enable browser-based kdapps and seamless "vibe coding," a robust client-server model is needed. Unlike the TTT example where each player is also a listener, this task involves designing patterns where the listener runs as a persistent backend (e.g., a web server). This allows clients (browsers) to interact via APIs, potentially using the Kaspa WASM SDK for client-side transaction generation. This architectural work is a foundational step for creating easy-to-deploy, end-to-end applications.

#### **State Management & Persistence**

This is a set of related tasks that build on each other to provide increasing levels of durability and history for kdapps.

  * `[ ]` **Stage 1: Short-Term Sync (RPC Catch-up):** The first step in persistence. This involves storing the latest sync point and implementing a catch-up process on startup. This allows a listener to recover from short downtimes without losing its place in the DAG (within Kaspa's pruning period).
  * `[ ]` **Stage 2: Full Reorg Protection (State Persistency):** Building on Stage 1, this task introduces a persistent database (e.g., RocksDB) to store rollback objects for arbitrary depths. This ensures that even very deep chain reorgs can be handled correctly. However, this stage does not solve for full history, as a new kdapp node would still be unable to sync a completed episode from scratch.
  * `[ ]` **Stage 3: Full History Sync (Archival Support):** The final stage, providing maximum flexibility. This involves designing a mechanism for dedicated archival nodes to store and serve the complete history of episodes. This would allow new kdapp nodes to join the network at any time and fully sync an episode's history, drawing inspiration from concepts like Kaspa's KIP-15 (ATAN).

#### **Advanced Research & Future Goals**

  * `[ ]` **Lay the Groundwork for AI-Assisted Development:** Building on the **decoupled client-server architecture**, the ultimate vision is to enable developers to generate `Episode` trait implementations from natural language prompts. This is a major exploration area with several key components:
      * `[ ]` **Generic Oracle Integration:** Designing a generic framework for feeding external data into an episode. This would involve defining an oracle via a special public key on episode initialization and creating a simple, pluggable system for integrating with various external data sources.
      * `[ ]` **Dynamic Code Loading:** Designing a server-side engine that can receive generated Rust code, compile it securely, and dynamically load it into a running kdapp backend.
      * `[ ]` **Multi-Language Support:** Exploring ways to support episodes written in other languages, such as Python. This would likely involve creating a generic Rust wrapper that can host and interact with foreign language runtimes.
      * `[ ]` **Compiler-Driven AI Feedback:** Leveraging the high-level, precise feedback from the Rust compiler (`rustc`) to create an automated loop where AI-generated code is refined and corrected based on compilation errors.
      * `[ ]` **Test Generation:** Developing prompt guides and methodologies for having the AI generate not just the episode logic, but also a corresponding suite of unit and integration tests to ensure correctness.