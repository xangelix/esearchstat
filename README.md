# eSearchStat

https://github.com/user-attachments/assets/placeholder-for-video

[![Crates.io](https://img.shields.io/crates/v/esearchstat)](https://crates.io/crates/esearchstat)
[![Docs.rs](https://docs.rs/esearchstat/badge.svg)](https://docs.rs/esearchstat)
[![License](https://img.shields.io/crates/l/esearchstat)](https://spdx.org/licenses/MIT)

**eSearchStat** is a high-performance desktop GUI for incredibly fast text searching, powered by ripgrep. It brings the legendary speed of professional command-line searching to everyday users through a simple, point-and-click interface. You don't need to be a programmer or wrestle with a confusing terminal—eSearchStat lets anyone hunt down specific words or phrases across thousands of documents, notes, and files as fast as your computer can possibly provide.

Unlike standard text search tools that grind to a halt on large filesystems, **eSearchStat** utilizes raw Rust concurrency underneath its UI. It combines a parallel, `.gitignore`-aware file crawler with optimized SIMD accelerated search regex matches, passing hits across lock-free pipelines. Results can be grouped into an interactive, collapsing directory tree or deep-filtered via secondary in-memory chain queries, while zero-copy binary layout maps allow you to save and instantly reload massive result datasets.

---

## 📸 Screenshots

(placeholder)

---

## 🚀 Key Features

- ⚡ **Asynchronous Streaming Engine:** Searches are offloaded to a background thread pool, instantly populating your interface with real-time streaming matches without blocking frame paints.
- 🌳 **Collapsible Structural Views:** Toggle seamlessly between a classic flat list view or a hierarchical directory tree mapping matching items by parent nodes.
- 🔍 **In-Memory Chained Search:** Narrow down thousands of active results instantly by launching a secondary regex query within existing memory buffers.
- 📦 **Zero-Copy Serialization:** Saves search dumps out to compressed `.ess` snapshots that utilize memory-mapped file casting via `bytemuck` for allocation-free reloading.

---

## 🚀 Installation & Build

### Prerequisites

Ensure you have the latest stable Rust toolchain installed.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/xangelix/esearchstat.git
cd esearchstat

# Build the release executable
cargo build --release

```

The compiled binary will be located at `target/release/esearchstat`.

---

## 📖 Usage Guide

Run the application from the command line:

```bash
./target/release/esearchstat

```

### Navigating the User Interface

1. **Configure Search Space:**
   Enter your query pattern inside the **Query** field (supports plain strings or standard regular expression syntaxes). Click **📁 Browse** to map your desired path target, or enter the path manually in the **Path** field.
2. **Fine-tune Controls:**
   Toggle target checkboxes to balance match constraints:

- **Ignore Case:** Switches regex evaluations over to case-insensitive pipelines.
- **Read Hidden:** Overrides defaults to search within dotfiles and concealed directories.
- **Max Results:** Sets strict collection ceilings to manage system memory overhead safely. You can hover over this element to see estimated RAM usage if maximum results are hit.

3. **Trigger Search Contexts:**
   Press `Enter` or click **🔍 New Search** to query. Alternatively, click **🔗 Chain Search** to re-filter the current results array using a brand new expression constraint.
4. **Inspect Matches:**
   Switch layout modes via the central toggles:

- **📝 Flat List View:** Displays every matched line matching absolute paths.
- **🌳 Collapsible Tree View:** Groups matches cleanly inside expandable directory folders.

5. **Contextual Action Menus:**

- **Left-Click:** Opens the streaming modal centered directly onto the chosen file's line match.
- **Right-Click:** Reveals a native pop-up context menu allowing you to open the parent directory inside your operating system's default file browser.
- **📋 Copy Button:** Generates and copies the exact command-line equivalent `ripgrep` expression string directly into your local clipboard.

---

## 💾 Saving & Loading Snapshots

Keep records of your codebase query states for future reference:

1. Once searches finish, click **File -> 💾 Save Results...** to serialize logs down to a structured `.ess` file.
2. To load historical datasets, go to **File -> 📖 Load Results...** to recall data states with full interactivity.
3. You can also output text summaries out to standard files by selecting **File -> 📤 Export Plain Text...**.

---

## 🛠 Architectural Design & Internals

### 1. Parallel Multi-Threaded Searcher (`src/core.rs`)

The matching engine hooks directly into the core systems that fuel `ripgrep`, bypassing terminal command injection forks:

- **Gitignore Awareness:** Using `ignore::WalkBuilder`, the background thread respects localized workspace configurations, automatically pruning heavy binaries, hidden paths, and nested `.gitignore` layouts.
- **SIMD Matching:** File matching uses `grep-searcher` and `grep-regex`, applying optimized platform primitives (AVX/SIMD acceleration) to chew through raw bytes.
- **Thread Pipeline Synchronization:** As threads find structural alignments, hits are funneled via non-blocking unbounded `crossbeam-channel` rings back to the main UI loop, which programmatically invokes a repaint signal `ctx.request_repaint()` to seamlessly render entries on screen.

### 2. Hierarchical View Compilation (`src/tree.rs`)

To present patterns clearly, **eSearchStat** supports structural tracking beyond regular list outputs through a multi-pass tokenization layout:

- **Path Component Insertion:** Incoming matches have relative parent paths split out by platform directory tokens. The structure recursively iterates components, inserting tracking nodes dynamically inside a strict `SearchTree` tree backed by a `BTreeMap`.
- **Virtualization Flattening Loop:** Rendering heavy trees usually kills UI performance. To keep thousands of visible layers at a steady FPS, the app continuously maps open directory branches into a flat data collection `FlatRow` enum. This collection passes directly into `egui::ScrollArea::show_rows` to lazily paint only elements current within view limits.

### 3. Zero-Copy Snapshot Persistence (`src/storage.rs`)

The `.ess` output layout mirrors raw system alignment memory rules to ensure reading structural indexes is virtually free:

```text
+------------------------------------------------------------+
|  Header (SearchResultsHeader - 24 Bytes)                   |
|  - Magic: "ESSTAT01"                                       |
|  - Match Count: u64                                        |
|  - Data Size (Arena Size): u64                             |
+------------------------------------------------------------+
|  Array of FileMatchEntry Structs (Flat Binary Segment)     |
|  - line_number: u64, path_offset/len, content_offset/len   |
+------------------------------------------------------------+
|  Data Arena (Raw Packed UTF-8 string bytes)                |
+------------------------------------------------------------+

```

- **Zero Allocation Casts:** The persistence layouts use `bytemuck::Pod` and `bytemuck::Zeroable` markers. When parsing a saved result dump, the program maps the file into memory via `memmap2::MmapOptions::map_copy`.
- **COW Mappings:** Data offsets reference a flat backend byte arena directly. This allows **eSearchStat** to reload indexes containing millions of matches in milliseconds, completely bypassing traditional processing overhead.

### 4. Lazy Streaming File Preview (`src/core.rs` & `src/gui.rs`)

Left-clicking any search result opens an interactive inline preview overlay tracking live file content states without loading full data models into system memory:

- **Mathematical Centering:** On the initial frame step, the layout calculates target heights using standard line variables, configuring initial viewport coordinates to perfectly capture and highlight target records.
- **Birectional Loading Blocks:** When users drag structural scroll bars to edges, the `FilePreviewer` fires isolated asynchronous I/O ranges to grab adjacent 40-line blocks.
- **Jitter Compensation:** Prepending line contents often pushes visible windows downward. The application avoids this jitter by taking added line heights and shifting offsets dynamically to maintain stable rendering.

---

## 📝 License

This project is licensed under the [MIT License](https://spdx.org/licenses/MIT).
