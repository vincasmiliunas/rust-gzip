#[link(name = "gzip", vers = "0.1", url = "https://github.com/vincasmiliunas/rust-gzip")];
#[crate_type = "lib"];
#[license = "zlib"];

extern mod std;

use std::libc::{c_long, c_ulong};

static ZLIB_VERSION: &'static str = "1.2.5\0";

pub enum FlushCode {
	Z_FINISH = 4
}

#[deriving(Eq, ToStr)]
pub enum ReturnCode {
	Z_OK = 0,
	Z_STREAM_END = 1,
	Z_NEED_DICT = 2,
	Z_ERRNO = -1,
	Z_STREAM_ERROR = -2,
	Z_DATA_ERROR = -3,
	Z_MEM_ERROR = -4,
	Z_BUF_ERROR = -5,
	Z_VERSION_ERROR = -6
}

pub enum CompressionLevel {
	Z_NO_COMPRESSION = 0,
	Z_BEST_SPEED = 1,
	Z_BALANCED_COMPRESSION = 5,
	Z_BEST_COMPRESSION = 9,
	Z_DEFAULT_COMPRESSION = -1
}

enum CompressionMethod {
	Z_DEFLATED = 8
}

enum WindowBits {
	GZIP_WBITS = 15+16
}

enum MemoryLevel {
	DEF_MEM_LEVEL = 8
}

enum CompressionStrategy {
	Z_DEFAULT_STRATEGY = 0
}

static GZIP_HEADER_BOUND: c_ulong = 16;

pub struct ZStream {
	next_in: *u8,
	avail_in: u32,
	total_in: c_ulong,
	
	next_out: *mut u8,
	avail_out: u32,
	total_out: c_ulong,
	
	msg: *u8,
	state: *u8,
	
	zalloc: *u8,
	zfree: *u8,
	opaque: *u8,
	
	data_type: u32,
	adler: c_ulong,
	reserved: c_ulong
}

pub struct GzHeader {
	text: i32,
	time: c_ulong,
	xflags: i32,
	os: i32,
	extra: *u8,
	extra_len: u32,
	extra_max: u32,
	name: *u8,
	name_max: u32,
	comment: *u8,
	comm_max: u32,
	hcrc: i32,
	done: i32
}

mod ffi {
	use std::libc::c_ulong;
	use super::*;
	
	#[link_args = "-lz"]
	extern {
		fn compressBound(src_len: c_ulong) -> c_ulong;
		fn deflateInit2_(strm: *mut ZStream, level: i32, method: i32, windowBits: i32, memLevel: i32, strategy: i32, version: *u8, stream_size: i32) -> i32;
		fn deflateSetHeader(strm: *mut ZStream, header: *mut GzHeader) -> i32;
		fn deflate(strm: *mut ZStream, flush: i32) -> i32;
		fn deflateEnd(strm: *mut ZStream) -> i32;
		fn inflateInit2_(strm: *mut ZStream, windowBits: i32, version: *u8, stream_size: i32) -> i32;
		fn inflate(strm: *mut ZStream, flush: i32) -> i32;
		fn inflateEnd(strm: *mut ZStream) -> i32;
	}
}

pub fn must_compress(input: &[u8]) -> ~[u8] {
	let ret = compress(input);
	if ret.is_err() {
		fail!("Gzip compression failed, code = %?", ret.unwrap_err());
	}
	return ret.unwrap();
}

pub fn compress(input: &[u8]) -> std::result::Result<~[u8], ReturnCode> {
	unsafe { compress_level(input, Z_DEFAULT_COMPRESSION) }
}

#[fixed_stack_segment] #[inline(never)]
pub unsafe fn compress_level(input: &[u8], level: CompressionLevel) -> std::result::Result<~[u8], ReturnCode> {
	let output_len = ffi::compressBound(input.len() as c_ulong) + GZIP_HEADER_BOUND;
	let mut output = std::vec::with_capacity(output_len as uint);
	let mut stream = ZStream {
		next_in: std::vec::raw::to_ptr(input), avail_in: input.len() as u32, total_in: 0,
		next_out: std::vec::raw::to_mut_ptr(output), avail_out: output_len as u32, total_out: 0,
		msg: std::ptr::null(), state: std::ptr::null(),
		zalloc: std::ptr::null(), zfree: std::ptr::null(), opaque: std::ptr::null(),
		data_type: 0, adler: 0, reserved: 0};
	let stream_ptr = std::ptr::to_mut_unsafe_ptr(&mut stream);
	
	let version = std::vec::raw::to_ptr(ZLIB_VERSION.as_bytes());
	let err1: ReturnCode = std::cast::transmute(ffi::deflateInit2_(stream_ptr, level as i32, Z_DEFLATED as i32,
		GZIP_WBITS as i32, DEF_MEM_LEVEL as i32, Z_DEFAULT_STRATEGY as i32, version, std::sys::size_of::<ZStream>() as i32) as c_long);
	if err1 != Z_OK {
		return std::result::Err(err1);
	}
	
	let mut gz_header = GzHeader {text: 0, time: 0, xflags: 0, os: 0, extra: std::ptr::null(), extra_len: 0, extra_max: 0, name: std::ptr::null(), name_max: 0, comment: std::ptr::null(), comm_max: 0, hcrc: 0, done: 0};
	let gz_header_ptr = std::ptr::to_mut_unsafe_ptr(&mut gz_header);
	let err2: ReturnCode = std::cast::transmute(ffi::deflateSetHeader(stream_ptr, gz_header_ptr) as c_long);
	if err2 != Z_OK {
		return std::result::Err(err2);
	}
	
	let err3: ReturnCode = std::cast::transmute(ffi::deflate(stream_ptr, Z_FINISH as i32) as c_long);
	let err4: ReturnCode = std::cast::transmute(ffi::deflateEnd(stream_ptr) as c_long);
	if err3 == Z_OK {
		return std::result::Err(Z_BUF_ERROR);
	} else if err3 != Z_STREAM_END {
		return std::result::Err(err3);
	} else if err4 == Z_OK {
		std::vec::raw::set_len(&mut output, stream.total_out as uint);
		return std::result::Ok(output);
	} else {
		return std::result::Err(err4);
	}
}

pub fn must_uncompress(input: &[u8]) -> ~[u8] {
	let ret = uncompress(input);
	if ret.is_err() {
		fail!("Gzip decompression failed, code = %?", ret.unwrap_err());
	}
	return ret.unwrap();
}

pub fn uncompress(input: &[u8]) -> std::result::Result<~[u8], ReturnCode> {
	unsafe { uncompress_config(input, 128*1024, 8) }
}

#[fixed_stack_segment] #[inline(never)]
pub unsafe fn uncompress_config(input: &[u8], initial_size: uint, multiplier: uint) -> std::result::Result<~[u8], ReturnCode> {
	let mut output = std::vec::with_capacity(initial_size);
	let mut stream = ZStream {
		next_in: std::vec::raw::to_ptr(input), avail_in: input.len() as u32, total_in: 0,
		next_out: std::vec::raw::to_mut_ptr(output), avail_out: output.capacity() as u32, total_out: 0,
		msg: std::ptr::null(), state: std::ptr::null(),
		zalloc: std::ptr::null(), zfree: std::ptr::null(), opaque: std::ptr::null(),
		data_type: 0, adler: 0, reserved: 0};
	let stream_ptr = std::ptr::to_mut_unsafe_ptr(&mut stream);
	
	let version = std::vec::raw::to_ptr(ZLIB_VERSION.as_bytes());
	let err1: ReturnCode = std::cast::transmute(ffi::inflateInit2_(stream_ptr, GZIP_WBITS as i32, version, std::sys::size_of::<ZStream>() as i32) as c_long);
	if err1 != Z_OK {
		return std::result::Err(err1);
	}

	loop {
		let err2: ReturnCode = std::cast::transmute(ffi::inflate(stream_ptr, Z_FINISH as i32) as c_long);
		if err2 == Z_BUF_ERROR && stream.avail_in > 0 {
			let buffer_size = output.capacity() * multiplier;
			let mut buffer = std::vec::with_capacity::<u8>(buffer_size);
			std::vec::raw::set_len(&mut buffer, stream.total_out as uint);
			std::vec::raw::set_len(&mut output, stream.total_out as uint);
			std::vec::bytes::copy_memory(buffer, output, stream.total_out as uint);
			output = buffer;
			let output_ptr = std::vec::raw::to_mut_ptr(output);
			stream.next_out = std::ptr::mut_offset(output_ptr, stream.total_out as int);
			stream.avail_out = buffer_size as u32 - stream.total_out as u32;
			loop;
		}
		
		let err3: ReturnCode = std::cast::transmute(ffi::inflateEnd(stream_ptr) as c_long);
		if err2 != Z_STREAM_END {
			return std::result::Err(err2);
		} else if err3 != Z_OK {
			return std::result::Err(err3);
		} else {
			std::vec::raw::set_len(&mut output, stream.total_out as uint);
			return std::result::Ok(output);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	struct Fixture {
		data: &'static str,
		level: CompressionLevel,
		initial_size: uint,
		multiplier: uint
	}
	
	static fixtures: &'static [Fixture] = &[
		Fixture { data: "b49546ca-a89b-4f6d-974a-a10d89495f4f",
			level: Z_DEFAULT_COMPRESSION, initial_size: 1024, multiplier: 2 },
		Fixture { data: "b49546ca-a89b-4f6d-974a-a10d89495f4f",
			level: Z_BEST_COMPRESSION, initial_size: 1, multiplier: 2 },
		Fixture { data: "b49546ca-a89b-4f6d-974a-a10d89495f4f",
			level: Z_BEST_SPEED, initial_size: 1, multiplier: 1024 },
		Fixture { data: "",
			level: Z_BALANCED_COMPRESSION, initial_size: 1024, multiplier: 2 },
		Fixture { data: "",
			level: Z_DEFAULT_COMPRESSION, initial_size: 1, multiplier: 2 },
		Fixture { data: "",
			level: Z_NO_COMPRESSION, initial_size: 1, multiplier: 1024 }];
	
	#[test]
	fn successes() {
		for &fixture in fixtures.iter() {
			let expected = fixture.data.as_bytes().to_owned();
			
			{
				let ret1 = unsafe { compress_level(expected, fixture.level) };
				assert!(ret1.is_ok());
				let compressed = ret1.unwrap();
				let ret2 = unsafe { uncompress_config(compressed, fixture.initial_size, fixture.multiplier) };
				assert!(ret2.is_ok());
				let actual = ret2.unwrap();
				assert!(expected == actual);
			}
			
			{
				let ret1 = compress(expected);
				assert!(ret1.is_ok());
				let compressed = ret1.unwrap();
				let ret2 = uncompress(compressed);
				assert!(ret2.is_ok());
				let actual = ret2.unwrap();
				assert!(expected == actual);
			}
			
			{
				let compressed = must_compress(expected);
				let actual = must_uncompress(compressed);
				assert!(expected == actual);
			}
		}
	}

	#[test]
	fn failures() {
		for &fixture in fixtures.iter() {
			let data = fixture.data.as_bytes().to_owned();
			
			let ret1 = unsafe { uncompress_config(data, fixture.initial_size, fixture.multiplier) };
			assert!(ret1.is_err());
			
			let ret2 = uncompress(data);
			assert!(ret2.is_err());
		}
	}
	
	#[test]
	fn gzip_fixture() {
		let compressed: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4b, 0x4c, 0x4a, 0xe6, 0x02, 0x00, 0x4e, 0x81, 0x88, 0x47, 0x04, 0x00, 0x00, 0x00];
		let uncompressed = "abc\n".as_bytes();
		{
			let ret = uncompress(compressed);
			assert!(ret.is_ok());
			let actual = ret.unwrap();
			assert!(uncompressed == actual);
		}
		{
			let ret = compress(uncompressed);
			assert!(ret.is_ok());
			let actual = ret.unwrap();
			assert!(compressed == actual);
		}
		{
			let actual = must_uncompress(compressed);
			assert!(uncompressed == actual);
		}
		{
			let actual = must_compress(uncompressed);
			assert!(compressed == actual);
		}
	}
}
