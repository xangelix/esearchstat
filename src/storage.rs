use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use bytemuck::{Pod, Zeroable};

use super::{core::SearchMatch, error::Error};

// --- Serializable Pod Structs for Bytemuck & Mmap (CoW) ---

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct SearchResultsHeader {
    pub magic: [u8; 8], // b"ESSTAT01"
    pub match_count: u64,
    pub data_size: u64,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct FileMatchEntry {
    pub line_number: u64,
    pub path_offset: u32,
    pub path_len: u32,
    pub content_offset: u32,
    pub content_len: u32,
}

pub fn save_results(matches: &[SearchMatch], path: &Path) -> Result<(), Error> {
    let mut arena = Vec::new();
    let mut entries = Vec::with_capacity(matches.len());

    for m in matches {
        let path_str = m.path.to_string_lossy();
        let path_bytes = path_str.as_bytes();
        let content_bytes = m.line_content.as_bytes();

        let path_offset = arena.len() as u32;
        arena.extend_from_slice(path_bytes);
        let path_len = path_bytes.len() as u32;

        let content_offset = arena.len() as u32;
        arena.extend_from_slice(content_bytes);
        let content_len = content_bytes.len() as u32;

        entries.push(FileMatchEntry {
            line_number: m.line_number,
            path_offset,
            path_len,
            content_offset,
            content_len,
        });
    }

    let header = SearchResultsHeader {
        magic: *b"ESSTAT01",
        match_count: entries.len() as u64,
        data_size: arena.len() as u64,
    };

    let mut file = File::create(path)?;
    file.write_all(bytemuck::bytes_of(&header))?;
    file.write_all(bytemuck::cast_slice(&entries))?;
    file.write_all(&arena)?;
    Ok(())
}

pub fn load_results(path: &Path) -> Result<Vec<SearchMatch>, Error> {
    let file = File::open(path)?;
    // Create Copy-on-Write (CoW) memory map
    let mmap = unsafe { memmap2::MmapOptions::new().map_copy(&file)? };

    let header_size = std::mem::size_of::<SearchResultsHeader>();
    if mmap.len() < header_size {
        return Err(Error::InvalidHeader);
    }

    let header: &SearchResultsHeader = bytemuck::from_bytes(&mmap[0..header_size]);
    if header.magic != *b"ESSTAT01" {
        return Err(Error::InvalidMagic);
    }

    let match_count = header.match_count as usize;
    let entry_size = std::mem::size_of::<FileMatchEntry>();
    let entries_start = header_size;
    let entries_end = entries_start + match_count * entry_size;

    if mmap.len() < entries_end {
        return Err(Error::FileTruncated("match entries"));
    }

    let entries: &[FileMatchEntry] = bytemuck::cast_slice(&mmap[entries_start..entries_end]);

    let data_start = entries_end;
    let data_end = data_start + header.data_size as usize;
    if mmap.len() < data_end {
        return Err(Error::FileTruncated("data arena"));
    }

    let arena = &mmap[data_start..data_end];
    let mut matches = Vec::with_capacity(match_count);

    for entry in entries {
        let path_start = entry.path_offset as usize;
        let path_end = path_start + entry.path_len as usize;
        let content_start = entry.content_offset as usize;
        let content_end = content_start + entry.content_len as usize;

        if path_end > arena.len() || content_end > arena.len() {
            return Err(Error::InvalidOffset);
        }

        let path_str = std::str::from_utf8(&arena[path_start..path_end])?;
        let content_str = std::str::from_utf8(&arena[content_start..content_end])?;

        matches.push(SearchMatch {
            path: PathBuf::from(path_str),
            line_number: entry.line_number,
            line_content: content_str.to_string(),
        });
    }

    Ok(matches)
}

// --- Serialization & Mmap Round-Trip Unit Test ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let test_matches = vec![
            SearchMatch {
                path: PathBuf::from("/path/to/file_a.rs"),
                line_number: 42,
                line_content: "let x = 123;".to_string(),
            },
            SearchMatch {
                path: PathBuf::from("/another/path/to/file_b.rs"),
                line_number: 101,
                line_content: "println!(\"Hello World\");".to_string(),
            },
        ];

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("esearchstat_test_results.ess");

        save_results(&test_matches, &file_path)?;

        let loaded_matches = load_results(&file_path)?;
        assert_eq!(loaded_matches.len(), test_matches.len());
        assert_eq!(loaded_matches[0].path, test_matches[0].path);
        assert_eq!(loaded_matches[0].line_number, test_matches[0].line_number);
        assert_eq!(loaded_matches[0].line_content, test_matches[0].line_content);
        assert_eq!(loaded_matches[1].path, test_matches[1].path);
        assert_eq!(loaded_matches[1].line_number, test_matches[1].line_number);
        assert_eq!(loaded_matches[1].line_content, test_matches[1].line_content);

        std::fs::remove_file(file_path)?;
        Ok(())
    }
}
