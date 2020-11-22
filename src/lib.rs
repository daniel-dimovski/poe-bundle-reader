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

pub fn read_index<F>(data: &[u8], callback: F)
where
    F: Fn(&Index, Vec<String>),
{
    unpack(&data, |bytes| {
        read_index_headers(&bytes, |index_data, remaining_bytes| {
            let mut c = Cursor::new(remaining_bytes);

            let mut generation_phase = false;
            let mut table = vec![];
            let mut files = vec![];

            while c.position() + 4 <= remaining_bytes.len() as u64 {
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

            callback(index_data, files);
        });
    });
}

fn read_utf8(c: &mut Cursor<&[u8]>) -> Result<String, FromUtf8Error> {
    let raw_bytes = (0..)
        .map(|_| c.read_u8().unwrap())
        .take_while(|&x| x != 0u8)
        .collect::<Vec<u8>>();
    return String::from_utf8(raw_bytes);
}

pub fn unpack<F>(data: &[u8], callback: F)
where
    F: Fn(&[u8]),
{
    let mut c = Cursor::new(data);

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

    let chunk_sizes = (0..chunk_count)
        .map(|_| c.read_u32::<LittleEndian>().unwrap())
        .map(|size| usize::try_from(size).unwrap())
        .collect::<Vec<usize>>();

    let mut chunk_offset = usize::try_from(c.position()).unwrap();
    let mut bytes_to_read = uncompressed_size;
    let mut output = Vec::with_capacity(usize::try_from(uncompressed_size).unwrap());

    (0..chunk_count as usize).for_each(|index| {
        let src = &data[chunk_offset..chunk_offset + chunk_sizes[index]];
        let dst_size = cmp::min(bytes_to_read, chunk_unpacked_size) as usize;
        let mut dst = vec![0u8; dst_size];

        let wrote = decompress(src.as_ptr(), chunk_sizes[index], dst.as_mut_ptr(), dst_size);
        if wrote < 0 {
            println!("Decompress returned: {} Expected: {}", wrote, dst_size);
            println!("first byte of index({}) {} {}", index, src[0], src[1]);
        }
        output.write(dst.as_slice()).unwrap();

        if bytes_to_read > chunk_unpacked_size {
            bytes_to_read -= chunk_unpacked_size;
        }
        chunk_offset = chunk_offset + chunk_sizes[index];
    });

    callback(output.as_slice());
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
}

// TODO: create structs for the index data and return
// TODO: split workload across cores
fn read_index_headers<F>(data: &[u8], callback: F)
where
    F: Fn(&Index, &[u8]),
{
    let mut c = Cursor::new(data);
    let bundle_count = c.read_u32::<LittleEndian>().unwrap();

    let bundles: HashMap<_, Bundle> = (0..bundle_count)
        .map(|index| {
            let name_length = c.read_u32::<LittleEndian>().unwrap();
            let name = (0..name_length)
                .map(|_| c.read_u8().unwrap())
                .collect::<Vec<u8>>();
            let uncompressed_size = c.read_u32::<LittleEndian>().unwrap();
            (
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

    let remainder = &data[c.position() as usize..];
    let index_data = Index {
        bundles,
        files,
        path_reps,
    };
    unpack(remainder, |bytes| {
        callback(&index_data, bytes);
    });
}
