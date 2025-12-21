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

pub enum MetaEvent {
    Unimplemented,
    EndOfTrack,
    SetTempo(u32)
}

enum Event {
    //TODO: fill in
    //sysex events aren't useful to us for our toy synth so we just skip them, they're basically just noops
    Sysex,
    Meta(MetaEvent),
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
    let mut cur: &[u8] = &track_buf;
    while cur.len() > 0 {
        // this will advance the slice past the delta time
        let delta_time = extract_vlq(&mut cur)?;
        let event = extract_event(&mut cur)?;
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
        // get next byte from slice and advance
        let b = extract_byte(bytes)?;
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

fn extract_byte(bytes: &mut &[u8]) -> Result<u8, Error> {
    let res = bytes.get(0).ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "unexpected EOF"));
    match res {
        Ok(v) => {
            *bytes = &bytes[1..];
            Ok(*v)
        }
        Err(e) => Err(e)
    }
}

fn extract_event(bytes: &mut &[u8]) -> Result<Event, Error> {
    let first = extract_byte(bytes)?;
    
    match first {
        0xf0 | 0xf7 => {
            let len = extract_vlq(bytes)? as usize;
            if bytes.len() < len {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "length vlq higher than actual bytes in sysex event!")
                );
            }
            *bytes = &bytes[len..];
            Ok(Event::Sysex)
        },
        0xff => extract_meta(bytes),
        _ => extract_midi(bytes)
    }
}


fn extract_meta(bytes: &mut &[u8]) -> Result<Event, Error> {
    let event_type = extract_byte(bytes)?;
    let len = extract_vlq(bytes)? as usize;
    if bytes.len() < len {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "length vlq higher than actual bytes in meta event!")
        );
    }

    // now we have to handle unimplemented events
    let meta_event = match event_type {
        0x2f => {
            if len != 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "EndOfTrack meta event must have length 0",
                ));
            }
            MetaEvent::EndOfTrack
        },
        0x51 => {
            // tempo is 3 bytes long always
            if len != 3 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "SetTempo meta event must have length 3",
                ));
            }
            
            // construct tempo, from_be_bytes requires 4 args and we can just set the most sig byte to be 0 cause it's big endian
            let tempo = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);

            MetaEvent::SetTempo(tempo)
        },
        0x01..=0x07 | 0x54 | 0x58 | 0x59 | 0x7f => MetaEvent::Unimplemented,
        _ => {
            return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid meta event tag"));
        }
    };

    *bytes = &bytes[len..];
    Ok(Event::Meta(meta_event))
}

fn extract_midi(bytes: &mut &[u8]) -> Result<Event, Error> {
    todo!("implement");
}


