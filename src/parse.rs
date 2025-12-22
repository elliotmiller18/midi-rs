use std::io::{BufReader, Read, Error, ErrorKind};
use std::path::Path;
use std::fs::File;
use crate::bits;

// MIDI SPEC: https://ccrma.stanford.edu/~craig/14q/midifile/MidiFileFormat.html
// or better: https://midimusic.github.io/tech/midispec.html
const HEADER_MARKER: u32 = 0x4d546864;
const TRACK_MARKER: u32 = 0x4d54726b;
const EXPECTED_INFO_SIZE_BYTES: usize = 6;
// midi event tags
const NOTE_OFF_STATUS: u8 = 0b1000;
const NOTE_ON_STATUS: u8 = 0b1001;
// unused midi event tags we have for skipping bytes
// const POLY_KEY_PRESSURE_STATUS: u8 = 0b1010;
// const CONTROL_CHANGE_STATUS: u8 = 0b1011;
const PROGRAM_CHANGE_STATUS: u8 = 0b1100;
const CHANNEL_PRESSURE_STATUS: u8 = 0b1101;
// const PITCH_WHEEL_CHANGE_STATUS: u8 = 0b1110;
const SYSTEM_MESSAGE_STATUS: u8 = 0b1111;

#[derive(PartialEq)]
enum FileFormat {
    SingleTrack,
    MultipleTrack,
    MultipleSong
}

#[derive(Debug)]
pub enum MetaEvent {
    Unimplemented,
    EndOfTrack,
    SetTempo(u32)
}

#[derive(Debug)]
pub enum MidiEvent {
    Unimplemented,
    NoteOn { note: u8, velocity: u8, channel: u8 } ,
    NoteOff { note: u8, velocity: u8, channel: u8 } ,
    //TODO: implement these, for now just note on and note off
    // ProgramChange(u8),
    // ControlChange(u8, u8),
    // PitchBend(u16),
}

#[derive(Debug)]
enum EventType {
    //sysex events aren't useful to us for our toy synth so we just skip them, they're basically just noops
    Sysex,
    Meta(MetaEvent),
    Midi(MidiEvent),
}

pub struct Event {
    ty: EventType,
    delta_time: u32
}

pub struct HeaderData {
    format: FileFormat,
    num_tracks: u16,
    // used for timing
    division: u16
}


pub fn parse(path: &Path) -> Result<(HeaderData, Vec<Event>), Error>
{
    assert!(path.exists());
    let file = File::open(path)?; 
    let mut reader = BufReader::new(file);

    Ok( (parse_header(&mut reader)?, parse_track(&mut reader)?) ) 
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

fn parse_track(reader: &mut BufReader<File>) -> Result<Vec<Event>, Error> {
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
    let mut running_status: Option<u8> = None;
    let mut events: Vec<Event> = vec![];
    while cur.len() > 0 {
        // this will advance the slice past the delta time
        let delta_time = extract_vlq(&mut cur)?;
        let ty = extract_event(&mut running_status, &mut cur)?;
        events.push(Event {ty, delta_time});
    }

    Ok(events)
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
        if !bits::msb_set(b) { break; }
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

fn extract_event(running_status: &mut Option<u8>, bytes: &mut &[u8]) -> Result<EventType, Error> {
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
            Ok(EventType::Sysex)
        },
        0xff => extract_meta(bytes),
        _ => extract_midi(running_status, first, bytes)
    }
}


fn extract_meta(bytes: &mut &[u8]) -> Result<EventType, Error> {
    let event_type = extract_byte(bytes)?;
    let len = extract_vlq(bytes)? as usize;
    if bytes.len() < len {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "length vlq higher than actual bytes in meta event!")
        );
    }

    let meta_event = match event_type {
        0x2f => {
            // end of track always has len 0 (it's the end of the track)
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
        // these are all of the meta events i'm not implementing cause they're not that interesting or super niche
        0x01..=0x07 | 0x54 | 0x58 | 0x59 | 0x7f => MetaEvent::Unimplemented,
        _ => {
            return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid meta event tag"));
        }
    };

    *bytes = &bytes[len..];
    Ok(EventType::Meta(meta_event))
}

// this func should only be called when we're at the start of a new event.
// therefore, we can just have the status either be the old running status or the current status byte?
fn extract_midi(running_status: &mut Option<u8>, first_byte: u8, bytes: &mut &[u8]) -> Result<EventType, Error> {
    // if the msb is set, this is a status marker meaning we need to update running_status,
    // if it is a data byte just check that there exists a running status! :D
    let mut using_running_status = running_status.is_some();
    if bits::msb_set(first_byte) {
        *running_status = Some(first_byte);
        using_running_status = false;
    } else if *running_status == None {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "somehow running status is none but we're looking at a data byte for our first byte")
        );
    }

    let status = running_status.ok_or_else(|| {
            Error::new(ErrorKind::InvalidData, "data byte with no running status")
    })?;

    match bits::msb(status) {
        NOTE_OFF_STATUS => Ok(EventType::Midi(MidiEvent::NoteOff { note: extract_byte(bytes)?, velocity: extract_byte(bytes)?, channel: bits::lsb(first_byte) })),
        NOTE_ON_STATUS => Ok(EventType::Midi(MidiEvent::NoteOn { note: extract_byte(bytes)?, velocity: extract_byte(bytes)?, channel: bits::lsb(first_byte) })),
        SYSTEM_MESSAGE_STATUS => {
            // skipping bits as appropriate for each system message on the tiny off chance they pop up
            match bits::lsb(status) {
                0b0000 => unreachable!("trying to handle sysex event tagged 0xF0 in extract_midi!"),
                0b0010 => {
                    // try and extract the next 2 bytes of unneeded system message data
                    extract_byte(bytes)?; extract_byte(bytes)?;
                }
                0b0011 => { extract_byte(bytes)?; }
                _ => {}
            }
            Ok(EventType::Midi(MidiEvent::Unimplemented))
        },
        PROGRAM_CHANGE_STATUS | CHANNEL_PRESSURE_STATUS => {
            // if we're not using running status, that means that the byte that was already extracted by the 
            // extract_event func is a status byte. if we are using running status, then this data byte
            // was already extracted by extract_event. same thing follows for the last arm below
            if !using_running_status { extract_byte(bytes)?; }
            // we've already consumed 
            Ok(EventType::Midi(MidiEvent::Unimplemented))
        }
        _ => {
            // see comment in above (PROGRAM_CHANGE_STATUS | CHANNEL_PRESSURE_STATUS) arm!
            if !using_running_status { extract_byte(bytes)?; }
            extract_byte(bytes)?;
            Ok(EventType::Midi(MidiEvent::Unimplemented))
        }
    }
}


