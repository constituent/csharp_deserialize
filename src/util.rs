use std::fs::File;
use std::io::prelude::*;

use byteorder::{LittleEndian, ReadBytesExt};

pub fn read_n_bytes(file: &mut File, n: usize) -> Vec<u8> {
	let mut buffer = vec![0; n];
	match file.read_exact(&mut buffer) {
		Ok(_) => buffer,
		_ => panic!("Reading file failed"),
	}
}

pub fn read_LengthPrefixedString(file: &mut File) -> String {
	let mut length: usize = 0;
	let mut byte_count: u8 = 0;
	loop {
		let current_length = file.read_u8().unwrap();
		
		if current_length > 0b01111111 {
			length += ((current_length & 0b01111111) as usize) << (byte_count * 7);
			byte_count += 1;
		} else {
			length += (current_length as usize) << (byte_count * 7);
			break;
		}
	}

	unsafe {
		String::from_utf8_unchecked(read_n_bytes(file, length))
	}
}

pub fn read_l_i32(file: &mut File) -> i32 {
	file.read_i32::<LittleEndian>().unwrap()
}
pub fn read_l_f32(file: &mut File) -> f32 {
	file.read_f32::<LittleEndian>().unwrap()
}
pub fn read_l_u64(file: &mut File) -> u64 {
	file.read_u64::<LittleEndian>().unwrap()
}
