use std::collections::HashMap;
use std::ops::Range;

use crate::terminal::model::grid::grid_handler::FILE_LINK_SEPARATORS;
use crate::terminal::ShellLaunchData;

cfg_if::cfg_if! {
    if #[cfg(feature = "local_fs")] {
        use std::collections::HashSet;
        use std::path::Path;
        use std::path::PathBuf;
        use warp_util::path::CleanPathResult;
    }
}

#[derive(Clone, Debug)]
#[cfg(feature = "local_fs")]
pub(crate) struct DetectedFilePath {
    pub absolute_path: PathBuf,
    pub line_and_column_num: Option<warp_util::path::LineAndColumnArg>,
}

#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
fn addr_of(s: &str) -> usize {
    s.as_ptr() as usize
}

/// Given a word with no whitespace in it, returns all the possible file paths within the word
/// from longest to shortest. File paths within a word can be split by a list of FILE_LINK_SEPARATORS,
/// and those separators may be part of file paths themselves.
/// Possible file paths begin after a separator and end before a separator.
/// For example, given /path/to/file:16:hello, it will return
/// ["/path/to/file:16:hello", "/path/to/file:16", "/path/to/file", "16:hello", "hello"]
#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
fn possible_file_paths_in_word(word: &str) -> impl Iterator<Item = &str> {
    // To include any substrings starting at the beginning of the word, we
    // pretend there's a separator before the first character.
    let mut separator_byte_indices = vec![-1];
    // We use char_indices() to get byte indices of each char which are used to index the string,
    // rather than chars().enumerate() would give char indices.
    for (i, c) in word.char_indices() {
        if FILE_LINK_SEPARATORS.contains(&c) {
            separator_byte_indices.push(i as i32);
        }
    }
    // Consider trailing periods to be separators. This is because
    // in natural language we might use a file path at the end of a sentence, and want
    // to detect them without including the trailing period. But trailing
    // periods can also be part of a valid file path.
    let word_ends_with_period = word.ends_with('.');
    if word_ends_with_period {
        separator_byte_indices.push((word.len() - 1) as i32);
    }
    // To include any substrings ending at the end of the word, we pretend there's
    // a separator after the last character.
    separator_byte_indices.push(word.len() as i32);
    let mut possible_path_byte_ranges = vec![];
    for (i, start_index) in separator_byte_indices.iter().cloned().enumerate() {
        for end_index in separator_byte_indices.iter().skip(i + 1).cloned() {
            if start_index + 1 < end_index {
                possible_path_byte_ranges.push(start_index + 1..end_index);
            }
        }
    }
    // Sort by longest to shortest.
    possible_path_byte_ranges.sort_by(|a, b| (b.end - b.start).cmp(&(a.end - a.start)));
    possible_path_byte_ranges
        .into_iter()
        .map(|range| &word[(range.start as usize)..(range.end as usize)])
}

/// Returns a DetectedLink::FilePath if expanded_path is a valid path that actually exists on the file system.
#[cfg(feature = "local_fs")]
fn compute_valid_file_path(
    working_directory: &Path,
    expanded_path: &str,
    files_and_folders_in_working_directory: &HashSet<PathBuf>,
    shell_launch_data: Option<&crate::terminal::ShellLaunchData>,
) -> Option<DetectedFilePath> {
    use crate::util::file::{absolute_path_if_valid, ShellPathType};
    // Scan for line and column number in the current word (left + right).
    let cleaned_path = CleanPathResult::with_line_and_column_number(expanded_path);

    // First try to use the files_and_folders_in_working_directory cache.
    let path = Path::new(&cleaned_path.path);
    if let Some(relative_path) = files_and_folders_in_working_directory.get(path) {
        let absolute_path = working_directory.join(relative_path);
        return Some(DetectedFilePath {
            absolute_path,
            line_and_column_num: cleaned_path.line_and_column_num,
        });
    } else if path.components().count() <= 1 {
        // If the path does not contain a separator and isn't in files_and_folders_in_working_directory,
        // we know it isn't a valid path. Return immediately to save a a file system call.
        return None;
    }

    // This does a file system lookup.
    let absolute_path = absolute_path_if_valid(
        &cleaned_path,
        ShellPathType::PlatformNative(working_directory.to_owned()),
        shell_launch_data,
    );

    absolute_path.map(|absolute_path| DetectedFilePath {
        absolute_path,
        line_and_column_num: cleaned_path.line_and_column_num,
    })
}

/// Returns a set of all file and folder names in the given directory (relative, not absolute paths).
#[cfg(feature = "local_fs")]
fn get_files_and_folders_in_directory(directory: &Path) -> HashSet<PathBuf> {
    let mut files_and_folders = HashSet::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return files_and_folders;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        files_and_folders.insert(PathBuf::from(entry.file_name()));
    }
    files_and_folders
}

/// Returns the detected valid file paths in some text along with their char ranges.
#[cfg(feature = "local_fs")]
pub(crate) fn detect_file_paths(
    working_directory: &str,
    text: &str,
    shell_launch_data: Option<&ShellLaunchData>,
) -> HashMap<Range<usize>, DetectedFilePath> {
    let mut file_paths = HashMap::new();
    // List files in this working_directory
    let working_directory = shell_launch_data
        .and_then(|launch_data| launch_data.maybe_convert_absolute_path(working_directory))
        .unwrap_or_else(|| {
            // Naively attempt to make a pathbuf from this.
            PathBuf::from(working_directory)
        });
    let files_and_folders_in_working_directory =
        get_files_and_folders_in_directory(working_directory.as_path());
    for word in text.split_whitespace() {
        let possible_paths = possible_file_paths_in_word(word);
        // In the word, there can be multiple valid file paths which may or may not overlap.
        // Take the longest one to turn into a link.
        for possible_path in possible_paths {
            // Need to expand the path here as built-in Path lib does not understand tilde.
            let expanded_path = shellexpand::tilde(possible_path);
            if let Some(path_type) = compute_valid_file_path(
                working_directory.as_path(),
                &expanded_path,
                &files_and_folders_in_working_directory,
                shell_launch_data,
            ) {
                let byte_start = addr_of(possible_path) - addr_of(text);
                let byte_end = byte_start + possible_path.len();
                let char_start = text[..byte_start].chars().count();
                let char_end = char_start + possible_path.chars().count();
                file_paths.insert(char_start..char_end, path_type.clone());

                // Check for line ranges after this file path and add them as separate clickable links
                if let Some(line_ranges) = detect_line_ranges_after_file_path(text, byte_end) {
                    for (line_number, char_range) in line_ranges {
                        // Create a new DetectedFilePath with the same file path but with the line number
                        let line_range_link = DetectedFilePath {
                            absolute_path: path_type.absolute_path.clone(),
                            line_and_column_num: Some(warp_util::path::LineAndColumnArg {
                                line_num: line_number as usize,
                                column_num: None,
                            }),
                        };
                        file_paths.insert(char_range, line_range_link);
                    }
                }

                break;
            }
        }
    }
    file_paths
}

use string_offset::CharOffset;
use warp_editor::content::buffer::Buffer;
use warpui::text::word_boundaries::WordBoundariesPolicy;

/// Returns the range of the word surrounding the given offset.
pub(crate) fn get_word_range_at_offset(
    buffer: &Buffer,
    offset: CharOffset,
    word_boundary_policy: Option<WordBoundariesPolicy>,
) -> Option<Range<CharOffset>> {
    use warp_editor::content::buffer::{ToBufferCharOffset, ToBufferPoint};
    use warpui::text::words::is_default_word_boundary;
    use warpui::text::TextBuffer;

    let word_boundary_policy = word_boundary_policy.unwrap_or(WordBoundariesPolicy::Default);
    let mut word_found_at: Option<CharOffset> = None;
    let mut cursor_offset = offset;

    if let Ok(chars) = buffer.chars_at(offset) {
        for c in chars {
            if c == '\n' {
                // Do not cross line boundaries when searching for the nearest word
                break;
            }
            if !is_default_word_boundary(c) {
                word_found_at = Some(cursor_offset);
                break;
            }
            // advance one character
            cursor_offset += 1;
        }
    }

    let found_offset = word_found_at?;
    let found_point = found_offset.to_buffer_point(buffer);

    let word_start_point = buffer
        .word_starts_backward_from_offset_inclusive(found_point)
        .ok()
        .map(|iter| iter.with_policy(&word_boundary_policy))
        .and_then(|mut iter| iter.next())
        .unwrap_or(found_point);

    let word_end_point = buffer
        .word_ends_from_offset_exclusive(found_point)
        .ok()
        .map(|iter| iter.with_policy(&word_boundary_policy))
        .and_then(|mut iter| iter.next())
        .unwrap_or(found_point);

    let word_start = word_start_point.to_buffer_char_offset(buffer);
    let word_end = word_end_point.to_buffer_char_offset(buffer);

    if word_start < word_end {
        Some(word_start..word_end)
    } else {
        None
    }
}

/// Parse line ranges from comma-separated text content and return detected ranges.
#[cfg(feature = "local_fs")]
fn parse_line_range(
    potential_range: &str,
    text: &str,
) -> Result<(u32, Range<usize>), &'static str> {
    let potential_range = potential_range.trim();

    // Look for pattern "number-number"
    let dash_pos = potential_range.find('-').ok_or("No dash found in range")?;

    // Extracting starting line number for potential range
    let start_str = potential_range[..dash_pos].trim();
    let start_line = start_str
        .parse::<u32>()
        .map_err(|_| "Failed to parse start line number")?;
    let end_str = potential_range[dash_pos + 1..].trim();
    end_str
        .parse::<u32>()
        .map_err(|_| "Failed to parse end line number")?;

    let range_start_bytes = addr_of(potential_range) - addr_of(text);
    let char_start = text[..range_start_bytes].chars().count();
    let range_end_bytes = range_start_bytes + potential_range.len();
    let char_end = text[..range_end_bytes].chars().count();

    Ok((start_line, char_start..char_end))
}

/// Helper function to detect line ranges that appear after a valid file path.
/// Looks for patterns like "file.rs (1-50, 100-150)" and returns the detected ranges.
/// Returns a vector of (line_number, char_range) tuples.
#[cfg(feature = "local_fs")]
fn detect_line_ranges_after_file_path(
    text: &str,
    file_path_byte_end: usize,
) -> Option<Vec<(u32, Range<usize>)>> {
    let chars_iter = text[file_path_byte_end..]
        .char_indices()
        .map(|(offs, ch)| (offs + file_path_byte_end, ch));

    // Finds an opening paranthesis, allowing some whitespace after file path, or returns None on failure
    let mut paren_start_idx = None;
    for (char_idx, ch) in chars_iter {
        if ch == '(' {
            paren_start_idx = Some(char_idx);
            break;
        } else if !ch.is_whitespace() {
            return None;
        }
    }
    let paren_start_idx = paren_start_idx?;

    // Find the matching closing paranthesis, or returns None on failure
    let paren_end_index = paren_start_idx + text[paren_start_idx..].find(')')?;

    // Extract the content between parentheses, and parse valid line ranges
    let paren_content = &text[paren_start_idx + 1..paren_end_index];
    let mut detected_ranges = Vec::new();

    for potential_range in paren_content.split(',') {
        match parse_line_range(potential_range, text) {
            Ok(range) => detected_ranges.push(range),
            Err(_) => return None,
        }
    }

    (!detected_ranges.is_empty()).then_some(detected_ranges)
}

#[cfg(test)]
#[path = "link_detection_test.rs"]
mod tests;
