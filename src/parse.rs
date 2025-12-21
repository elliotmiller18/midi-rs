use std::io::{BufReader, Read, Error, ErrorKind};
use std::path::Path;
use std::fs::File;

// MIDI SPEC: https://ccrma.stanford.edu/~craig/14q/midifile/MidiFileFormat.html
// or better: https://midimusic.github.io/tech/midispec.html
const HEADER_MARKER: u32 = 0x4d546864;
const TRACK_MARKER: u32 = 0x4d54726b;
const EXPECTED_INFO_SIZE_BYTES: usize = 6;

enum FileFormat {
    SingleTrack,
    MultipleTrack,
    MultipleSong
}

pub struct SysexEventData {
    //TODO: fill in
}

enum Event {
    //TODO: fill in
    Sysex,
    Meta,
    MIDI,

}

pub struct TrackChunk {
    event: Event,
    delta_time: u32
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
    let mut reader = BufReader::new(file);

    let header_data = parse_header(&mut reader)?;
    parse_tracks(&mut reader);

    Ok(())
}

fn parse_header(reader: &mut BufReader<File>) -> Result<HeaderData, Error>
{   
    let mut marker_buf = [0u8; 4];
    reader.read_exact(&mut marker_buf)?;
    
    if u32::from_be_bytes(marker_buf) != HEADER_MARKER {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "trying to read malformed file - header bytes missing")
        );
    }

    let mut length_buf = [0u8; 4];
    reader.read_exact(&mut length_buf)?;
    // info size should always be 6 bytes
    if u32::from_be_bytes(length_buf) != EXPECTED_INFO_SIZE_BYTES.try_into().unwrap() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "trying to read malformed file - header chunk size different from expected")
        );
    }

    let mut data_buf = [0u8; EXPECTED_INFO_SIZE_BYTES];
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

    // this is ugly looking, basically all we're saying here is the num_tracks is bytes 2 and 3 are the number of tracks
    // and that the division (delta time unit) is bytes 4 and 5. 
    let num_tracks = u16::from_be_bytes(data_buf[2..4].try_into().unwrap());
    let division = u16::from_be_bytes(data_buf[4..6].try_into().unwrap());
    
    Ok( HeaderData { format, num_tracks, division } )
}

fn parse_tracks(reader: &mut BufReader<File>) -> Result<Vec<TrackChunk>, Error> {
    let mut marker_buf = [0u8; 4];
    reader.read_exact(&mut marker_buf)?;
    
    if u32::from_be_bytes(marker_buf) != TRACK_MARKER {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "trying to read malformed file - track header bytes missing")
        );
    }

    //TODO: wrap this all in a while loop, for now we just read one track
    let mut length_buf: [u8; 4] = [0u8; 4];
    reader.read_exact(&mut length_buf)?;
    
    // there are only 32 bytes for the size but vec! needs a usize
    let length = u32::from_be_bytes(length_buf) as usize;
    let mut track_buf = vec![0u8; length];
    reader.read_exact(&mut track_buf)?;
    while track_buf.len() > 0 {
        let mut delta_slice: &[u8] = &track_buf;
        // this will advance the slice past the delta time
        let delta_time = extract_vlq(&mut delta_slice)?;
        //TODO: grab events
    }

    //TODO: remove placeholder value
    Ok(vec![TrackChunk {event: Event::Sysex, delta_time: 0}])
}

// midi files use this interesting (weird) encoding i haven't seen before, see this for more:
// https://midimusic.github.io/tech/midispec.html#BM1_1
fn extract_vlq(bytes: &mut &[u8]) -> Result<u32, Error> {

    let mut vlq = 0;
    for i in 0..4 {
        // get next byte from slice 
        let b = *bytes.get(0).ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "EOF in VLQ"))?;
        // advance slice by 1 
        *bytes = &bytes[1..];
        // shift variable_length 7 to the left and add the current byte without its msb to varible_length
        vlq = (vlq << 7) | u32::from(b & 0x7f);
        // the msb being a 0 indiciates that this is the final byte of data
        if (b & 0x80) == 0 { break; }
        // the file is claiming there's more data outside of the allowed 4 byte range
        else if i == 3 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "VLQ too long")
            );
        }
    }

    Ok(vlq)
}

fn extract_event(bytes: &mut &[u8]) {

}

