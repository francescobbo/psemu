use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::ops::Range;
use std::path::Path;

use crate::cdrom::{
    CdTime,
    cuebin::{CdBinFiles, CueSheet, TrackType},
};

const SECTOR_HEADER_LEN: u64 = 16;

const MODE_1_DIGEST_RANGE: Range<usize> = 0..2064;
const MODE_1_CHECKSUM_LOCATION: Range<usize> = 2064..2068;

const MODE_2_SUBMODE_LOCATION: usize = 18;

const MODE_2_FORM_1_DIGEST_RANGE: Range<usize> = 16..2072;
const MODE_2_FORM_1_CHECKSUM_LOCATION: Range<usize> = 2072..2076;

const MODE_2_FORM_2_DIGEST_RANGE: Range<usize> = 16..2348;
const MODE_2_FORM_2_CHECKSUM_LOCATION: Range<usize> = 2348..2352;

type CdBinFsFiles = CdBinFiles<File>;

#[derive(Debug)]
enum CdRomReader {
    CueBin(CdBinFsFiles),
}

impl Default for CdRomReader {
    fn default() -> Self {
        Self::CueBin(CdBinFiles::empty())
    }
}

impl CdRomReader {
    fn read_sector(
        &mut self,
        track_number: u8,
        relative_sector_number: u32,
        out: &mut [u8],
    ) {
        match self {
            Self::CueBin(bin_files) => {
                bin_files.read_sector(track_number, relative_sector_number, out)
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdRomFileFormat {
    // CUE file + BIN files
    CueBin,
}

impl CdRomFileFormat {
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        match path.as_ref().extension().and_then(OsStr::to_str) {
            Some("cue") => Some(Self::CueBin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode2Form {
    // 2048-byte sector with ECC bytes
    One,
    // 2324-byte sector with no ECC bytes, only EDC
    Two,
}

impl Mode2Form {
    fn parse(sector_buffer: &[u8]) -> Self {
        // Submode bit 5 specifies Form 1 vs. Form 2
        if sector_buffer[MODE_2_SUBMODE_LOCATION] & (1 << 5) != 0 {
            Self::Two
        } else {
            Self::One
        }
    }
}

#[derive(Debug)]
pub struct CdRom {
    cue_sheet: CueSheet,
    reader: CdRomReader,
}

impl CdRom {
    /// Open a CD-ROM reader that will read from the filesystem as needed.
    ///
    /// # Errors
    ///
    /// Will propagate any I/O errors, and will return an error if the CD-ROM metadata appears
    /// invalid.
    pub fn open<P: AsRef<Path>>(path: P, format: CdRomFileFormat) -> Self {
        match format {
            CdRomFileFormat::CueBin => Self::open_cue_bin(path),
        }
    }

    fn open_cue_bin<P: AsRef<Path>>(cue_path: P) -> Self {
        let (bin_files, cue_sheet) =
            CdBinFiles::create(cue_path, |path| File::open(path));

        Self {
            cue_sheet,
            reader: CdRomReader::CueBin(bin_files),
        }
    }

    #[must_use]
    pub fn cue(&self) -> &CueSheet {
        &self.cue_sheet
    }

    /// Read a 2352-byte sector from the given track into a buffer.
    ///
    /// # Errors
    ///
    /// This method will propagate any I/O error encountered while reading from disk.
    ///
    /// # Panics
    ///
    /// This method will panic if `out`'s length is less than 2352 or if `relative_time` is past the
    /// end of the track file.
    pub fn read_sector(
        &mut self,
        track_number: u8,
        relative_time: CdTime,
        out: &mut [u8],
    ) {
        let track = self.cue_sheet.track(track_number);
        if relative_time < track.pregap_len
            || relative_time
                >= track.end_time - track.postgap_len - track.start_time
        {
            // Reading data in pregap or postgap that does not exist in the file
            match track.track_type {
                TrackType::Data => {
                    write_fake_data_pregap(relative_time, out);
                }
                TrackType::Audio => {
                    // Fill with all 0s
                    out[..2352 as usize].fill(0);
                }
            }
        }

        let relative_sector_number =
            (relative_time - track.pregap_len).to_sector_number();
        self.reader
            .read_sector(track_number, relative_sector_number, out);

        // validate_edc(track.mode, track_number, relative_sector_number, out)?;

        // TODO check P/Q ECC?
    }
}

fn write_fake_data_pregap(time: CdTime, out: &mut [u8]) {
    // Make up a header; 12 sync bytes, then minutes, then seconds, then frames, then mode (always 1)
    let bcd_minutes = time_component_to_bcd(time.minutes);
    let bcd_seconds = time_component_to_bcd(time.seconds);
    let bcd_frames = time_component_to_bcd(time.frames);
    out[..SECTOR_HEADER_LEN as usize].copy_from_slice(&[
        0x00,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x11,
        0x00,
        bcd_minutes,
        bcd_seconds,
        bcd_frames,
        0x01,
    ]);
    out[SECTOR_HEADER_LEN as usize..2352 as usize].fill(0);
}

fn time_component_to_bcd(component: u8) -> u8 {
    let msb = component / 10;
    let lsb = component % 10;
    (msb << 4) | lsb
}
