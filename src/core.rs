use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    thread,
};

use crossbeam_channel::Sender;
use eframe::egui;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{Searcher, Sink, SinkMatch};
use ignore::WalkBuilder;

/// The structured data sent across our async pipeline channel.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub path: PathBuf,
    pub line_number: u64,
    pub line_content: String,
}

/// Our custom `Sink` implementation.
/// The `Searcher` calls our `matched` method for each hit it finds.
struct GuiSink {
    path: PathBuf,
    tx: Sender<SearchMatch>,
    count: usize,
    limit: usize,
}

impl Sink for GuiSink {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if self.count >= self.limit {
            return Ok(false); // Stop searching this file
        }

        // Read the matched slice from the searcher's internal memory buffer
        let line_content = std::str::from_utf8(mat.bytes())
            .unwrap_or("")
            .trim_end()
            .to_string();

        // Push the result over our non-blocking channel to the GUI thread
        let _ = self.tx.send(SearchMatch {
            path: self.path.clone(),
            line_number: mat.line_number().unwrap_or(0),
            line_content,
        });

        self.count += 1;
        // Return `true` to instruct the engine to keep searching this file
        Ok(true)
    }
}

/// Spawns a background thread to walk the directory and search the files.
pub fn start_search(
    query: String,
    target_path: PathBuf,
    ignore_case: bool,
    read_hidden: bool,
    limit: usize,
    tx: Sender<SearchMatch>,
    ctx: egui::Context,
) {
    thread::spawn(move || {
        // Compile the regex query using ripgrep's regex matcher builder
        let Ok(matcher) = RegexMatcherBuilder::new()
            .case_insensitive(ignore_case)
            .build(&query)
        else {
            return; // Exit if the regular expression is invalid
        };

        // Initialize the parallel, .gitignore-aware directory walker
        let walker = WalkBuilder::new(target_path).hidden(!read_hidden).build();
        let mut searcher = Searcher::new();
        let mut total_matches = 0usize;

        for result in walker {
            if total_matches >= limit {
                break; // Stop directory walk early
            }

            let Ok(entry) = result else {
                continue;
            };

            // Only process physical file entries (skips folders, symlinks, etc.)
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                let path = entry.path().to_path_buf();
                let mut sink = GuiSink {
                    path: path.clone(),
                    tx: tx.clone(),
                    count: total_matches,
                    limit,
                };

                // Search the file; results are pushed immediately into the Sink
                let _ = searcher.search_path(&matcher, &path, &mut sink);
                total_matches = sink.count;

                // Instruct egui to repaint the screen as results flow in
                ctx.request_repaint();
            }
        }
    });
}

// --- Streaming File Window Loader ---

pub fn read_line_range(
    path: &Path,
    start_line: usize,
    end_line: usize,
) -> std::io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_num = idx + 1;
        if line_num >= start_line && line_num <= end_line {
            lines.push(line?);
        }
        if line_num > end_line {
            break;
        }
    }
    Ok(lines)
}

pub struct FilePreviewer {
    pub path: PathBuf,
    pub target_line: usize,
    pub loaded_range: std::ops::Range<usize>, // 1-based, start..end
    pub lines: Vec<String>,
    pub error: Option<String>,
    pub needs_scroll_to_target: bool,
    pub current_scroll_offset: f32,
    pub pending_scroll_adjustment: Option<f32>,
    pub scroll_settled_delay: usize,
}

impl FilePreviewer {
    #[must_use]
    pub fn new(path: PathBuf, target_line: usize) -> Self {
        let mut previewer = Self {
            path,
            target_line,
            loaded_range: 0..0,
            lines: Vec::new(),
            error: None,
            needs_scroll_to_target: true,
            current_scroll_offset: 0.0,
            pending_scroll_adjustment: None,
            scroll_settled_delay: 5,
        };
        previewer.initial_load();
        previewer
    }

    fn initial_load(&mut self) {
        let start = if self.target_line > 40 {
            self.target_line - 40
        } else {
            1
        };
        let end = self.target_line + 40;
        match read_line_range(&self.path, start, end) {
            Ok(lines) => {
                self.lines = lines;
                self.loaded_range = start..(start + self.lines.len());
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn load_more_above(&mut self, count: usize) {
        if self.loaded_range.start <= 1 {
            return;
        }
        let start = if self.loaded_range.start > count {
            self.loaded_range.start - count
        } else {
            1
        };
        let end = self.loaded_range.start - 1;
        match read_line_range(&self.path, start, end) {
            Ok(new_lines) => {
                let len = new_lines.len();
                self.lines.splice(0..0, new_lines);
                self.loaded_range = (self.loaded_range.start - len)..self.loaded_range.end;
                #[allow(clippy::cast_precision_loss)]
                let len_f = len as f32;
                self.pending_scroll_adjustment = Some(len_f);
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn load_more_below(&mut self, count: usize) {
        let start = self.loaded_range.end;
        let end = start + count;
        match read_line_range(&self.path, start, end) {
            Ok(new_lines) => {
                if new_lines.is_empty() {
                    return;
                }
                let len = new_lines.len();
                self.lines.extend(new_lines);
                self.loaded_range = self.loaded_range.start..(self.loaded_range.end + len);
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
}
