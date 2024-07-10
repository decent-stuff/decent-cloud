use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::{read, Compression};
use std::io;
use std::io::{Read, Write};

// Compress a string and return a Zlib Encoded vector of bytes or error
pub fn zlib_compress(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(bytes)?;
    e.finish()
}

// Decompress a Zlib Encoded vector of bytes and return a string or error
pub fn zlib_decompress(bytes: &[u8]) -> io::Result<String> {
    let mut gz = read::ZlibDecoder::new(bytes);
    let mut s = String::new();
    gz.read_to_string(&mut s)?;
    Ok(s)
}

// Compress a string and return a Gzip Encoded vector of bytes or error
pub fn gzip_compress(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(bytes)?;
    e.finish()
}

// Decompress a Gzip Encoded vector of bytes and return a string or error
pub fn gzip_decompress(bytes: &[u8]) -> io::Result<String> {
    let mut gz = read::GzDecoder::new(bytes);
    let mut s = String::new();
    gz.read_to_string(&mut s)?;
    Ok(s)
}
