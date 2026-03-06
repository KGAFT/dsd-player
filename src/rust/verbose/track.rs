use dsf::DsfFile;
use id3::{Tag, TagLike};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io;
use std::fs::File;
use std::io::Cursor;
use gstreamer::glib::GString;
use gstreamer_pbutils::gst;
use crate::verbose::text_decoder;

#[derive(Debug, Default)]
pub struct DsdiffMetadata {
    /// Native tags, e.g., DIAR, DITI
    pub tags: HashMap<String, String>,
    /// Optional raw ID3 chunk
    pub id3_raw: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct Track {
    pub name: String,
    pub album_name: String,
    pub artist_name: String,
    pub genre: String,
    pub path: String,
    pub year: u32,
    pub bit_depth: u8,
    pub sample_rate: u32,
    pub channels: u8,
    pub overall_bitrate: u32,
    pub cover_bytes: Vec<u8>,
    pub offset_ms: u64,
    pub duration_ms: u64,
}

impl Track {
    pub fn gst_new(path: &str) -> Option<Track> {
        use gstreamer_pbutils::gst::Tag;
        let _ = gst::init();
        let uri = gst::glib::filename_to_uri(path, None).unwrap();

        let discoverer = match gstreamer_pbutils::Discoverer::new(gst::ClockTime::from_seconds(5)) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("Discoverer failed");
                return None;
            }
        };

        let info = match discoverer.discover_uri(&uri) {
            Ok(i) => i,
            Err(_) => {
                eprintln!("Info failed {}", uri);
                return None;
            }
        };

        let tags = match info.tags() {
            Some(t) => t,
            None => {
                eprintln!("Tags failed!");
                return None;
            }
        };

        let file_name = Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let get_tag = |tag| unsafe {
            use gstreamer::glib::value::FromValue;
            use gstreamer::glib::value::ToValue;
            let res = tags.generic(tag)?;
            let res = res.to_value();
            Some(GString::from_value(&res).to_string())
        };

        let title = get_tag(gst::tags::Title::TAG_NAME).unwrap_or(file_name.clone());
        let album = get_tag(gst::tags::Album::TAG_NAME).unwrap_or_else(|| {
            Path::new(path)
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        });
        let artist = get_tag(gst::tags::Artist::TAG_NAME).unwrap_or_default();
        let genre = get_tag(gst::tags::Genre::TAG_NAME).unwrap_or_default();

        let duration_ms = info.duration().unwrap_or_default().mseconds();
        let overall_bitrate = info.audio_streams().first().map(|s| s.bitrate()).unwrap_or(0);

        Some(Track {
            name: title,
            album_name: album,
            artist_name: artist,
            genre,
            path: path.to_string(),
            duration_ms,
            overall_bitrate,
            ..Default::default()
        })
    }

    pub fn new(path: &str) -> Option<Track> {
        let path_buf = PathBuf::from(path);
        let f_name = path_buf.file_name().unwrap().to_str().unwrap().to_string();

        if f_name.ends_with(".dsf") || f_name.ends_with(".dff") || f_name.ends_with(".dsd") {
            let trad_res = Self::extract_dsd_info(path);
            if let Some(trad) = trad_res {
                return Some(trad);
            } else {
                return Some(Self::gst_new(path).unwrap_or(Self::empty_track(path)));
            }
        }
        return None;
    }

    pub fn try_get_dsd_tag_trad(path: &str) -> Option<Tag> {
        let dsf = match DsfFile::open(Path::new(path)) {
            Ok(f) => f,
            Err(_) => {
                eprintln!("Failed to open dsd file {}", path);
                return None;
            }
        };
        match dsf.id3_tag() {
            Some(t) => Some(t.clone()),
            None => {
                eprintln!("Failed to read dsd file id3 tag {}", path);
                None
            }
        }
    }

    pub fn extract_dff_metadata(path: &str) -> io::Result<DsdiffMetadata> {
        use std::io::{Read, Seek, SeekFrom};
        use byteorder::{BigEndian, ReadBytesExt};

        let mut file = File::open(path)?;
        let mut metadata = DsdiffMetadata {
            tags: HashMap::new(),
            id3_raw: None,
        };

        // DSDIFF uses 64-bit big-endian chunk sizes (unlike standard 32-bit IFF).
        // Top-level layout: "FRM8" (4B) | size (8B u64 BE) | form type "DSD " (4B) | sub-chunks...
        let mut id = [0u8; 4];
        file.read_exact(&mut id)?;
        if &id != b"FRM8" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a DSDIFF file: missing FRM8"));
        }
        let frm8_size = file.read_u64::<BigEndian>()?;

        let mut form_type = [0u8; 4];
        file.read_exact(&mut form_type)?;
        if &form_type != b"DSD " {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a DSDIFF file: wrong form type"));
        }

        // FRM8 header = 4 (id) + 8 (size) = 12 bytes; frm8_size counts from byte 12 onward.
        let frm8_end: u64 = 12 + frm8_size;

        loop {
            let pos = file.seek(SeekFrom::Current(0))?;
            if pos + 12 > frm8_end {
                break;
            }

            let mut chunk_id = [0u8; 4];
            if file.read_exact(&mut chunk_id).is_err() {
                break;
            }
            let chunk_size = match file.read_u64::<BigEndian>() {
                Ok(s) => s,
                Err(_) => break,
            };
            let chunk_data_pos = file.seek(SeekFrom::Current(0))?;

            match &chunk_id {
                b"DIIN" => {
                    // DIIN contains metadata sub-chunks: DIAR, DITI, DIAL, DIGN, etc.
                    // Each text sub-chunk is: UINT32 count (BE) + count bytes of text.
                    let diin_end = chunk_data_pos + chunk_size;
                    loop {
                        let sub_pos = file.seek(SeekFrom::Current(0))?;
                        if sub_pos + 12 > diin_end {
                            break;
                        }
                        let mut sub_id = [0u8; 4];
                        if file.read_exact(&mut sub_id).is_err() {
                            break;
                        }
                        let sub_size = match file.read_u64::<BigEndian>() {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        let sub_data_pos = file.seek(SeekFrom::Current(0))?;

                        if let Ok(sub_id_str) = std::str::from_utf8(&sub_id) {
                            if let Some(key) = Self::normalize_dff_tag(sub_id_str) {
                                if sub_size >= 4 {
                                    if let Ok(text_len) = file.read_u32::<BigEndian>() {
                                        let read_len = (text_len as u64).min(sub_size - 4) as usize;
                                        let mut text_bytes = vec![0u8; read_len];
                                        if file.read_exact(&mut text_bytes).is_ok() {
                                            let text = text_decoder::binary_to_text(&text_bytes)
                                                .trim()
                                                .to_string();
                                            metadata.tags.insert(key.into(), text);
                                        }
                                    }
                                }
                            }
                        }

                        // Chunks are padded to even byte boundaries.
                        let padded = sub_size + (sub_size & 1);
                        if file.seek(SeekFrom::Start(sub_data_pos + padded)).is_err() {
                            break;
                        }
                    }
                }
                b"ID3 " => {
                    let mut raw = vec![0u8; chunk_size as usize];
                    if file.read_exact(&mut raw).is_ok() {
                        metadata.id3_raw = Some(raw);
                    }
                }
                _ => {}
            }

            let padded = chunk_size + (chunk_size & 1);
            if file.seek(SeekFrom::Start(chunk_data_pos + padded)).is_err() {
                break;
            }
        }

        Ok(metadata)
    }

    fn normalize_dff_tag(id: &str) -> Option<&'static str> {
        match id {
            "DIAR" => Some("artist"),
            "DITI" => Some("title"),
            "DIAL" => Some("album"),
            "DIGN" => Some("genre"),
            "DICR" => Some("copyright"),
            "DIFC" => Some("comment"),
            _ => None,
        }
    }

    pub fn extract_dsd_info(path: &str) -> Option<Track> {
        let mut tag_res = Self::try_get_dsd_tag_trad(path);
        if tag_res.is_none() {
            eprintln!("Opening in another way");
            if path.ends_with("dff"){
                let data = Self::extract_dff_metadata(path).unwrap();
                tag_res = id3::v1v2::read_from(Cursor::new(data.id3_raw.unwrap())).ok();
            }
            if tag_res.is_none() {
                eprintln!("No way to get to dsd data: {}", path);
                return None;
            }
        }
        let tag = tag_res.unwrap();

        let path_buf = PathBuf::from(path);
        let f_name = path_buf.file_name().unwrap().to_str().unwrap().to_string();

        let sample_rate: u32 = 0;
        let channels: u8 = 0;
        let bit_depth: u8 = 1;
        let year = tag.year().unwrap_or(0) as u32;
        let duration_ms = tag.duration().unwrap_or(0) as u64;
        let overall_bitrate = (sample_rate as u64 * channels as u64 * bit_depth as u64) as u32;

        let cover_bytes = tag
            .pictures()
            .next()
            .map(|pic| pic.data.to_vec())
            .unwrap_or_default();

        Some(Track {
            name: tag.title().unwrap_or(f_name.as_str()).to_string(),
            album_name: tag.album().unwrap_or("").to_string(),
            artist_name: tag.artist().unwrap_or("").to_string(),
            genre: tag.genre().unwrap_or("").to_string(),
            path: path.to_string(),
            sample_rate,
            channels,
            bit_depth,
            year,
            duration_ms,
            overall_bitrate,
            cover_bytes,
            ..Default::default()
        })
    }

    pub fn empty_track(path: &str) -> Track {
        Track {
            name: PathBuf::from(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: path.to_string(),
            ..Default::default()
        }
    }
}