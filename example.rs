#[link(name = "example", vers = "0.1")];

extern mod std;
mod gzip;

fn main() {
	let args = std::os::args();
	for arg in args.iter() {
		let compressed = gzip::must_compress(arg.as_bytes());
		let uncompressed = gzip::must_uncompress(compressed);
		let arg2 = std::str::from_utf8(uncompressed);
		println(fmt!("arg = %s, compressed = %?, uncompressed = %s", *arg, compressed, arg2));
	}
}