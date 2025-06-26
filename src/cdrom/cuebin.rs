use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::OnceLock;
use std::{fs, io, mem};

use regex::Regex;

use crate::cdrom::CdTime;

#[derive(Debug, Clone)]
pub struct TrackMetadata {
    pub file_name: String,
    pub time_in_file: CdTime,
}

#[derive(Debug)]
struct CdRomFile<F: Read + Seek> {
    file: BufReader<F>,
    position: u64,
}

impl<F: Read + Seek> CdRomFile<F> {
    fn new(file: F) -> Self {
        Self {
            file: BufReader::new(file),
            position: 0,
        }
    }
}

#[derive(Debug)]
pub struct CdBinFiles<F: Read + Seek> {
    files: HashMap<String, CdRomFile<F>>,
    track_metadata: Vec<TrackMetadata>,
}

impl<F: Read + Seek> CdBinFiles<F> {
    pub fn empty() -> Self {
        Self {
            files: HashMap::new(),
            track_metadata: Vec::new(),
        }
    }

    pub fn create<OpenFn, P: AsRef<Path>>(
        cue_path: P,
        bin_open_fn: OpenFn,
    ) -> (Self, CueSheet)
    where
        OpenFn: for<'a> Fn(&'a Path) -> io::Result<F>,
    {
        let cue_path = cue_path.as_ref();

        let (cue_sheet, track_metadata) = parse_cue(cue_path);

        let file_names: HashSet<_> = track_metadata
            .iter()
            .map(|metadata| metadata.file_name.clone())
            .collect();

        let parent_dir = cue_path.parent().unwrap();

        let mut files = HashMap::with_capacity(file_names.len());
        for file_name in file_names {
            let file_path = parent_dir.join(Path::new(&file_name));
            let file = bin_open_fn(&file_path).unwrap_or_else(|_| {
                panic!(
                    "Failed to open track file '{file_name}' referenced in CUE file '{}'",
                    cue_path.display()
                )
            });
            files.insert(file_name, CdRomFile::new(file));
        }

        let bin_files = Self {
            files,
            track_metadata,
        };
        (bin_files, cue_sheet)
    }

    pub fn read_sector(
        &mut self,
        track_number: u8,
        relative_sector_number: u32,
        out: &mut [u8],
    ) -> () {
        let metadata = &self.track_metadata[(track_number - 1) as usize];
        let CdRomFile {
            file: track_file,
            position,
        } = self
            .files
            .get_mut(&metadata.file_name)
            .expect("Track file was not opened on load; this is a bug");

        let sector_number = metadata.time_in_file.to_sector_number()
            + relative_sector_number
            - 150;
        let sector_addr = u64::from(sector_number) * 2352;

        // Only seek if the file descriptor is not already at the desired position
        // println!(
        //     "Seeking to sector {sector_number} at address {sector_addr:08x}"
        // );
        if *position != sector_addr {
            track_file.seek(SeekFrom::Start(sector_addr)).unwrap_or_else(|_| {
                panic!(
                    "Failed to seek to sector {sector_number} in track file '{}'",
                    metadata.file_name
                )
            });
        }

        track_file
            .read_exact(&mut out[..2352 as usize])
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to read sector {sector_number} from track file '{}'",
                    metadata.file_name
                )
            });
        *position = sector_addr + 2352;
    }
}

#[derive(Debug, Clone)]
struct ParsedTrack {
    number: u8,
    mode: TrackMode,
    pregap_len: Option<CdTime>,
    pause_start: Option<CdTime>,
    track_start: CdTime,
}

#[derive(Debug, Clone)]
struct ParsedFile {
    file_name: String,
    tracks: Vec<ParsedTrack>,
}

#[derive(Debug, Clone)]
struct CueParser {
    files: Vec<ParsedFile>,
    tracks: Vec<ParsedTrack>,
    current_file: Option<String>,
    current_track: Option<(u8, TrackMode)>,
    last_track_number: Option<u8>,
    pregap_len: Option<CdTime>,
    pause_start: Option<CdTime>,
    track_start: Option<CdTime>,
}

impl CueParser {
    fn new() -> Self {
        Self {
            files: vec![],
            tracks: vec![],
            current_file: None,
            current_track: None,
            last_track_number: None,
            pregap_len: None,
            pause_start: None,
            track_start: None,
        }
    }

    fn parse(mut self, file: &str) -> Vec<ParsedFile> {
        for line in file.lines() {
            if line.starts_with("FILE ") {
                self.parse_file_line(line);
            } else if line.starts_with("  TRACK ") {
                self.parse_track_line(line);
            } else if line.starts_with("    INDEX ") {
                self.parse_index_line(line);
            } else if line.starts_with("    PREGAP ") {
                self.parse_pregap_line(line);
            }
        }

        self.push_file();

        if self.files.is_empty() {
            panic!("No files found in CUE file; this is a bug");
        }

        self.files
    }

    fn parse_file_line(&mut self, line: &str) {
        static RE: OnceLock<Regex> = OnceLock::new();

        self.push_file();

        let re =
            RE.get_or_init(|| Regex::new(r#"FILE "(.*)" BINARY"#).unwrap());
        let captures = re
            .captures(line)
            .ok_or_else(|| panic!("Invalid file line: {line}"))
            .unwrap();
        let file_name = captures.get(1).unwrap();
        self.current_file = Some(file_name.as_str().into());
    }

    fn parse_track_line(&mut self, line: &str) {
        static RE: OnceLock<Regex> = OnceLock::new();

        self.push_track();

        let re =
            RE.get_or_init(|| Regex::new(r"TRACK ([^ ]*) ([^ ]*)").unwrap());
        let captures = re
            .captures(line)
            .ok_or_else(|| panic!("Invalid track line: {line}"))
            .unwrap();
        let track_number =
            captures.get(1).unwrap().as_str().parse::<u8>().unwrap();
        let mode = captures
            .get(2)
            .unwrap()
            .as_str()
            .parse::<TrackMode>()
            .unwrap();

        self.current_track = Some((track_number, mode));
    }

    fn parse_index_line(&mut self, line: &str) {
        static RE: OnceLock<Regex> = OnceLock::new();

        let re =
            RE.get_or_init(|| Regex::new(r"INDEX ([^ ]*) ([^ ]*)").unwrap());
        let captures = re
            .captures(line)
            .ok_or_else(|| panic!("Invalid index line: {line}"))
            .unwrap();
        let index_number = captures.get(1).unwrap();
        let start_time =
            captures.get(2).unwrap().as_str().parse::<CdTime>().unwrap();

        match index_number.as_str() {
            "00" => {
                self.pause_start = Some(start_time);
            }
            "01" => {
                self.track_start = Some(start_time);
            }
            _ => {
                panic!(
                    "Unexpected index number '{index_number:?}' in CUE file line: {line}"
                );
            }
        }
    }

    fn parse_pregap_line(&mut self, line: &str) {
        static RE: OnceLock<Regex> = OnceLock::new();

        let re = RE.get_or_init(|| Regex::new(r"PREGAP ([^ ]*)").unwrap());
        let captures = re
            .captures(line)
            .ok_or_else(|| panic!("Invalid pregap line: {line}"))
            .unwrap();
        let pregap_len =
            captures.get(1).unwrap().as_str().parse::<CdTime>().unwrap();

        self.pregap_len = Some(pregap_len);
    }

    fn push_file(&mut self) {
        self.push_track();

        let Some(current_file) = self.current_file.take() else {
            return;
        };

        if self.tracks.is_empty() {
            panic!(
                "No tracks found in CUE file for file '{current_file}'; this is a bug"
            );
        }

        self.files.push(ParsedFile {
            file_name: current_file,
            tracks: mem::take(&mut self.tracks),
        });
    }

    fn push_track(&mut self) {
        let Some((track_number, track_mode)) = self.current_track.take() else {
            return;
        };

        match self.last_track_number {
            None => {
                if track_number != 1 {
                    panic!("Expected first track to be 01, was {track_number}");
                }
            }
            Some(last_track_number) => {
                if track_number != last_track_number + 1 {
                    panic!(
                        "Tracks out of order; track {track_number} after {last_track_number}"
                    );
                }
            }
        }
        self.last_track_number = Some(track_number);

        let Some(track_start) = self.track_start.take() else {
            panic!("No start time found for track {track_number}");
        };

        self.tracks.push(ParsedTrack {
            number: track_number,
            mode: track_mode,
            pregap_len: self.pregap_len.take(),
            pause_start: self.pause_start.take(),
            track_start,
        });
    }
}

fn parse_cue<P: AsRef<Path>>(cue_path: P) -> (CueSheet, Vec<TrackMetadata>) {
    let cue_path = cue_path.as_ref();

    let cue_file = fs::read_to_string(cue_path).unwrap();
    let parsed_files = CueParser::new().parse(&cue_file);

    to_cue_sheet(parsed_files, cue_path)
}

fn to_cue_sheet(
    parsed_files: Vec<ParsedFile>,
    cue_path: &Path,
) -> (CueSheet, Vec<TrackMetadata>) {
    let cue_parent_dir = cue_path.parent().unwrap_or_else(|| {
        panic!("CUE file '{cue_path:?}' has no parent directory; this is a bug")
    });

    let mut absolute_start_time = CdTime::ZERO;
    let mut tracks = Vec::new();
    let mut track_metadata = Vec::new();

    for ParsedFile {
        file_name,
        tracks: parsed_tracks,
    } in parsed_files
    {
        let bin_path = cue_parent_dir.join(&file_name);

        let file_metadata = fs::metadata(&bin_path).unwrap();
        let file_len_bytes = file_metadata.len();
        let file_len_sectors = (file_len_bytes / 2352) as u32;

        for i in 0..parsed_tracks.len() {
            let track = &parsed_tracks[i];

            let track_type = track.mode.to_type();
            let pregap_len = match track_type {
                TrackType::Data => {
                    // Data tracks always have a 2-second pregap
                    CdTime::new(0, 2, 0)
                }
                TrackType::Audio => track.pregap_len.unwrap_or(CdTime::ZERO),
            };
            let pause_len =
                track.pause_start.map_or(CdTime::ZERO, |pause_start| {
                    track.track_start - pause_start
                });

            let is_last_track_in_file = i == parsed_tracks.len() - 1;
            let data_end_time = if is_last_track_in_file {
                CdTime::from_sector_number(file_len_sectors)
            } else {
                let next_track = &parsed_tracks[i + 1];
                next_track.pause_start.unwrap_or(next_track.track_start)
            };

            let postgap_len = track_type.default_postgap_len();

            let padded_track_len = pregap_len
                + pause_len
                + (data_end_time - track.track_start)
                + postgap_len;
            tracks.push(Track {
                number: track.number,
                mode: track.mode,
                track_type,
                start_time: absolute_start_time,
                end_time: absolute_start_time + padded_track_len,
                pregap_len,
                pause_len,
                postgap_len,
            });
            track_metadata.push(TrackMetadata {
                file_name: file_name.clone(),
                time_in_file: track.pause_start.unwrap_or(track.track_start),
            });

            absolute_start_time += padded_track_len;
        }
    }

    finalize_track_list(&mut tracks);
    (CueSheet::new(tracks), track_metadata)
}

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    Data,
    Audio,
}

impl TrackType {
    #[must_use]
    pub(crate) fn default_postgap_len(self) -> CdTime {
        match self {
            // Data tracks always have a 2-second postgap
            Self::Data => CdTime::new(0, 2, 0),
            Self::Audio => CdTime::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackMode {
    Mode1,
    Mode2,
    Audio,
}

impl TrackMode {
    #[must_use]
    pub fn to_type(self) -> TrackType {
        match self {
            Self::Mode1 | Self::Mode2 => TrackType::Data,
            Self::Audio => TrackType::Audio,
        }
    }
}

impl FromStr for TrackMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MODE1/2352" => Ok(Self::Mode1),
            "MODE2/2352" => Ok(Self::Mode2),
            "AUDIO" => Ok(Self::Audio),
            _ => Err(format!("unsupported CD track type: {s}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub number: u8,
    pub mode: TrackMode,
    pub track_type: TrackType,
    pub start_time: CdTime,
    pub end_time: CdTime,
    pub pregap_len: CdTime,
    pub pause_len: CdTime,
    pub postgap_len: CdTime,
}

impl Track {
    #[must_use]
    pub fn effective_start_time(&self) -> CdTime {
        self.start_time + self.pregap_len + self.pause_len
    }
}

#[derive(Debug, Clone)]
pub struct CueSheet {
    tracks: Vec<Track>,
    track_start_times: Vec<CdTime>,
}

impl CueSheet {
    /// Create a new `CueSheet` from the given track list.
    ///
    /// # Panics
    ///
    /// This function will panic if the track list is empty.
    #[must_use]
    pub(crate) fn new(tracks: Vec<Track>) -> Self {
        assert!(!tracks.is_empty(), "track list must not be empty");

        let track_start_times =
            tracks.iter().map(|track| track.start_time).collect();

        Self {
            tracks,
            track_start_times,
        }
    }

    #[must_use]
    pub fn track(&self, track_number: u8) -> &Track {
        &self.tracks[(track_number - 1) as usize]
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn last_track(&self) -> &Track {
        self.tracks.last().unwrap()
    }

    /// Find the track containing the specified time. Returns `None` if the time is past the end of
    /// the disc.
    #[must_use]
    pub fn find_track_by_time(&self, time: CdTime) -> Option<&Track> {
        match self.track_start_times.binary_search(&time) {
            Ok(i) => Some(&self.tracks[i]),
            Err(i) => {
                if i < self.tracks.len() {
                    Some(&self.tracks[i - 1])
                } else {
                    let last_track = self.last_track();
                    (time <= last_track.end_time).then_some(last_track)
                }
            }
        }
    }
}

#[must_use]
pub(crate) fn tracks_are_continuous(tracks: &[Track]) -> bool {
    if tracks[0].start_time != CdTime::ZERO {
        return false;
    }

    for window in tracks.windows(2) {
        let [track, next] = window else {
            unreachable!("windows(2)")
        };
        if next.start_time != track.end_time {
            return false;
        }
    }

    true
}

pub(crate) fn finalize_track_list(tracks: &mut [Track]) {
    // The final track always has a 2-second postgap
    let last_track = tracks.last_mut().unwrap();
    if last_track.postgap_len == CdTime::ZERO {
        last_track.postgap_len = CdTime::new(0, 2, 0);
        last_track.end_time += CdTime::new(0, 2, 0);
    }
}
