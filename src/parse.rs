use std::io::{BufReader, Read, Error, ErrorKind};
use std::path::Path;
use std::fs::File;

// MIDI SPEC: https://ccrma.stanford.edu/~craig/14q/midifile/MidiFileFormat.html
const HEADER_MARKER: u32 = 0x4d546864;
const INFO_SIZE_BYTES: usize = 6;
const MARKER_LEN_BYTES: usize = 4;
const HEADER_LEN_BYTES: usize = 4;

enum FileFormat {
    SingleTrack,
    MultipleTrack,
    MultipleSong
}

pub struct HeaderData {
    format: FileFormat,
    num_tracks: u16,
    // used for timing
    division: u16
}


pub fn parse(path: &Path) -> Result<(), Error>
{
    assert!(path.exists());
    let file = File::open(path)?;
    let header_data = parse_header(file)?;
    Ok(())
}

fn parse_header(file: File) -> Result<HeaderData, Error>
{   
    let mut reader = BufReader::new(file);
    let mut marker_buf = [0u8; MARKER_LEN_BYTES];

    reader.read_exact(&mut marker_buf)?;
    
    if u32::from_be_bytes(marker_buf) != HEADER_MARKER {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "trying to read malformed file - header bytes missing")
        );
    }

    let mut length_buf = [0u8; HEADER_LEN_BYTES];
    reader.read_exact(&mut length_buf)?;
    // info size should always be 6 bytes
    if u32::from_be_bytes(length_buf) != INFO_SIZE_BYTES.try_into().unwrap() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "trying to read malformed file - header chunk size different from expected")
        );
    }

    let mut data_buf = [0u8; INFO_SIZE_BYTES];
    reader.read_exact(&mut data_buf)?;

    let format = match u16::from_be_bytes(data_buf[0..2].try_into().unwrap()) {
        0 => FileFormat::SingleTrack,  
        1 => FileFormat::MultipleTrack,
        2 => FileFormat::MultipleSong,
        _ => {  return Err(Error::new(
                    ErrorKind::InvalidData,
                    "trying to read malformed file - file format number is invalid")
                );
            }
    };

    let num_tracks = u16::from_be_bytes(data_buf[2..4].try_into().unwrap());
    let division = u16::from_be_bytes(data_buf[2..4].try_into().unwrap());
    
    // this is ugly looking, basically all we're saying here is the num_tracks is bytes 2 and 3 are the number of tracks
    // and that the division (delta time unit) is bytes 4 and 5. 
    Ok( HeaderData { format, num_tracks, division } )
}