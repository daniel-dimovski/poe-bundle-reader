use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::io::{Cursor, Write};
use std::string::FromUtf8Error;

extern crate libc;
use std::{cmp, str};

// bundle implementation discussion
// https://github.com/poe-tool-dev/ggpk.discussion/wiki/Bundle-scheme

#[link(name = "libooz", kind = "static")]
extern "C" {
    fn Ooz_Decompress(src_buf: *const u8, src_len: u32, dst: *mut u8, dst_size: usize) -> i32;
}

fn decompress(source: *const u8, src_len: usize, destination: *mut u8, dst_size: usize) -> i32 {
    // TODO: at some point look into implementing the decompression in rust
    unsafe {
        return Ooz_Decompress(source, src_len as u32, destination, dst_size);
    }
}

pub struct Bundle {
    pub name: String,
    pub uncompressed_size: u32,
}

pub struct File {
    pub bundle_index: u32,
    pub offset: u32,
    pub size: u32,
}

pub struct PathRep {
    pub payload_offset: u32,
    pub payload_size: u32,
    pub payload_recursive_size: u32,
}

pub struct Index {
    pub bundles: HashMap<u32, Bundle>,
    pub files: HashMap<u64, File>,
    pub path_reps: HashMap<u64, PathRep>,
    pub paths: Vec<String>,
}

// --- internal to DatFile::source
// let reader = poe_bundle_reader.read('/games/Path of Exile/') // returns an object for grabbing data
// let bytes = reader.get("Data/Mods.dat") // return uncompressed &[u8] of the file
// TODO: make them lazy + cached (optional?)

// ---------------------
// let dat_reader = DatFile::source("/games/Path of Exile/") // creates poe_bundle_reader object
// dat.read("Data/Mods.dat") // DatFile object (with access to the dat_reader internally for following links)

pub fn read_index(data: &[u8]) -> Index {
    let size = unpack(&data, &mut Vec::with_capacity(0));
    let mut dst = Vec::with_capacity(size);
    unpack(&data, &mut dst);
    build_index(&dst)
}

pub fn unpack(src: &[u8], dst: &mut Vec<u8>) -> usize {
    let mut c = Cursor::new(src);

    let _ = c.read_u32::<LittleEndian>().unwrap(); // total size (uncompressed)
    let _ = c.read_u32::<LittleEndian>().unwrap(); // total size (compressed)
    let _ = c.read_u32::<LittleEndian>().unwrap(); // head size

    let _ = c.read_u32::<LittleEndian>().unwrap(); // encoding of first chunk
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let uncompressed_size = c.read_u64::<LittleEndian>().unwrap();
    let _ = c.read_u64::<LittleEndian>().unwrap(); // total size (compressed)
    let chunk_count = c.read_u32::<LittleEndian>().unwrap();
    let chunk_unpacked_size = c.read_u32::<LittleEndian>().unwrap() as u64;
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown
    let _ = c.read_u32::<LittleEndian>().unwrap(); // unknown

    if dst.capacity() < uncompressed_size as usize {
        return uncompressed_size as usize;
    }

    let chunk_sizes = (0..chunk_count)
        .map(|_| c.read_u32::<LittleEndian>().unwrap())
        .map(|size| usize::try_from(size).unwrap())
        .collect::<Vec<usize>>();

    let mut chunk_offset = usize::try_from(c.position()).unwrap();
    let mut bytes_to_read = uncompressed_size;

    // multithread decompression
    (0..chunk_count as usize).for_each(|index| {
        let src = &src[chunk_offset..chunk_offset + chunk_sizes[index]];
        let dst_size = cmp::min(bytes_to_read, chunk_unpacked_size) as usize;

        let mut chunk_dst = vec![0u8; dst_size + 64];
        let wrote = decompress(src.as_ptr(), chunk_sizes[index], chunk_dst.as_mut_ptr(), dst_size);
        if wrote < 0 {
            println!("Decompress returned: {} Expected: {}", wrote, dst_size);
            println!("first byte of index({}) {} {}", index, src[0], src[1]);
        }
        dst.write(&chunk_dst[0..dst_size]).unwrap();

        if bytes_to_read > chunk_unpacked_size {
            bytes_to_read -= chunk_unpacked_size;
        }
        chunk_offset = chunk_offset + chunk_sizes[index];
    });
    return 0;
}

fn build_index(data: &[u8]) -> Index {
    let mut c = Cursor::new(data);
    let bundle_count = c.read_u32::<LittleEndian>().unwrap();

    let bundles: HashMap<_, Bundle> = (0..bundle_count)
        .map(|index| {
            let name_length = c.read_u32::<LittleEndian>().unwrap();
            let name = (0..name_length)
                .map(|_| c.read_u8().unwrap())
                .collect::<Vec<u8>>();
            let uncompressed_size = c.read_u32::<LittleEndian>().unwrap();
            ( // TODO: clean up
                index,
                Bundle {
                    name: str::from_utf8(name.as_slice()).unwrap().to_string(),
                    uncompressed_size,
                },
            )
        })
        .collect();

    let file_count = c.read_u32::<LittleEndian>().unwrap();
    let files: HashMap<_, File> = (0..file_count)
        .map(|_| {
            let hash = c.read_u64::<LittleEndian>().unwrap();
            (
                hash,
                File {
                    bundle_index: c.read_u32::<LittleEndian>().unwrap(),
                    offset: c.read_u32::<LittleEndian>().unwrap(),
                    size: c.read_u32::<LittleEndian>().unwrap(),
                },
            )
        })
        .collect();

    let path_rep_count = c.read_u32::<LittleEndian>().unwrap();
    let path_reps: HashMap<_, PathRep> = (0..path_rep_count)
        .map(|_| {
            let hash = c.read_u64::<LittleEndian>().unwrap();
            (
                hash,
                PathRep {
                    payload_offset: c.read_u32::<LittleEndian>().unwrap(),
                    payload_size: c.read_u32::<LittleEndian>().unwrap(),
                    payload_recursive_size: c.read_u32::<LittleEndian>().unwrap(),
                },
            )
        })
        .collect();

    let remaining_bytes = &data[c.position() as usize..];
    let size = unpack(&remaining_bytes, &mut Vec::with_capacity(0));
    let mut dst = Vec::with_capacity(size);
    unpack(&remaining_bytes, &mut dst);

    Index {
        bundles,
        files,
        path_reps,
        paths: build_paths(dst.as_slice()),
    }
}

fn build_paths(bytes: &[u8]) -> Vec<String> {
    let mut c = Cursor::new(bytes);

    let mut generation_phase = false;
    let mut table = vec![];
    let mut files = vec![];

    while c.position() + 4 <= bytes.len() as u64 {
        let index = c.read_u32::<LittleEndian>().unwrap() as usize;

        if index == 0 {
            generation_phase = !generation_phase;
            if generation_phase {
                table.clear();
            }
        }

        if index > 0 {
            let mut text = read_utf8(&mut c).unwrap();
            if index <= table.len() {
                text = format!("{}{}", table[index - 1], text);
            }

            if generation_phase {
                table.push(text)
            } else {
                files.push(text);
            }
        }
    }
    files
}

fn read_utf8(c: &mut Cursor<&[u8]>) -> Result<String, FromUtf8Error> {
    let raw_bytes = (0..)
        .map(|_| c.read_u8().unwrap())
        .take_while(|&x| x != 0u8)
        .collect::<Vec<u8>>();
    return String::from_utf8(raw_bytes);
}
